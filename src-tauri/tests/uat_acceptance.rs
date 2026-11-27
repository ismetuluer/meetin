//! Otomatik UAT Kabul Test Paketi (Faz E).
//!
//! Hedef uygulama matrisi (**Word `WINWORD.EXE`**, **Excel `EXCEL.EXE`**,
//! **Chrome `chrome.exe`**, **Slack `slack.exe`**, **Notepad `notepad.exe`**,
//! **Windows Terminal `WindowsTerminal.exe`**) üzerinde:
//! - Hedef programa göre yapıştırma stratejisi ve `{imleç}` geri ok adımları,
//! - Uygulama kuralları (`only` / `exclude`) ve SQLite kalıcılığı,
//! - Türkçe büyük/küçük harf uyumu (3 mod + `Shift+F3` + `Aksanları yoksay`),
//! - Biçimli HTML → `CF_HTML`, `RTF` ve düz metin dönüşümü,
//! - Seçim filtresi ve parola alanı güvenliği,
//! - `.mee` ZIP zengin medya arşivi gidiş-dönüşü ve bozulma koruması,
//! - İmzalı güncelleme manifesti (`ECDSA P-256` + `SHA-256`) doğrulaması.
//!
//! Tüm senaryolar deterministik olarak sınanır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{Local, TimeZone};
use tempfile::TempDir;

use meetin_lib::db::{FolderRecord, NewPhrase, PhraseRecord, Storage};
use meetin_lib::hook::buffer::KeyBuffer;
use meetin_lib::hook::{is_bubble_hidden_process, is_process_excluded};
use meetin_lib::inject::clipboard::PasteShortcut;
use meetin_lib::inject::rich::{build_cf_html, html_to_plain_text, html_to_rtf};
use meetin_lib::inject::{
    build_cursor_left_steps, build_reselect_left_count, build_short_text_sequence,
    paste_shortcut_for_process, should_use_clipboard, InjectStep, SHORT_TEXT_THRESHOLD,
};
use meetin_lib::io::csv::ConflictStrategy;
use meetin_lib::io::formats::{export_meetin_json, import_items, parse_import_file};
use meetin_lib::io::mee_archive::{
    export_mee_archive, import_mee_archive, is_zip_signature, MeeArchiveError,
};
use meetin_lib::matcher::case::{
    apply_case, cycle_turkish_case, normalize_abbr, tr_lower, tr_title, tr_upper, CaseMode,
};
use meetin_lib::matcher::{ActiveContext, AppRule, AppRuleKind, Matcher, PhraseEntry};
use meetin_lib::selection::{
    clean_selected_text, resolve_selection_outcome, SelectionSourceOutcome,
};
use meetin_lib::vars::{extract_form_fields, resolve_template, ResolveContext};

const TARGET_MATRIX: &[(&str, &str, bool, PasteShortcut)] = &[
    // (process_name, sample_window_title, prefers_clipboard_on_short_text, paste_shortcut)
    ("WINWORD.EXE", "Belge1 - Word", false, PasteShortcut::CtrlV),
    ("EXCEL.EXE", "Kitap1 - Excel", false, PasteShortcut::CtrlV),
    (
        "chrome.exe",
        "Web Posta - Google Chrome",
        false,
        PasteShortcut::CtrlV,
    ),
    (
        "slack.exe",
        "Genel Kanal - Slack",
        false,
        PasteShortcut::CtrlV,
    ),
    (
        "notepad.exe",
        "Adsız - Not Defteri",
        true,
        PasteShortcut::CtrlV,
    ),
    (
        "WindowsTerminal.exe",
        "PowerShell - Windows Terminal",
        false,
        PasteShortcut::CtrlShiftV,
    ),
];

fn make_entry(id: i64, abbr: &str, content: &str, case_mode: CaseMode) -> PhraseEntry {
    PhraseEntry {
        id,
        abbr: abbr.to_string(),
        content: content.to_string(),
        content_html: None,
        case_mode,
        app_rules: Vec::new(),
        enabled: true,
    }
}

#[test]
fn uat_target_app_matrix_injection_strategy_and_cursor() {
    let short_len = 48;
    let boundary_len = SHORT_TEXT_THRESHOLD;
    let long_len = SHORT_TEXT_THRESHOLD + 1;

    for &(proc_name, _title, short_uses_clip, expected_shortcut) in TARGET_MATRIX {
        assert_eq!(
            should_use_clipboard(short_len, Some(proc_name)),
            short_uses_clip,
            "kısa metinde pano tercihi hatalı: {proc_name}"
        );
        assert_eq!(
            should_use_clipboard(boundary_len, Some(proc_name)),
            short_uses_clip,
            "eşik uzunlukta pano tercihi hatalı: {proc_name}"
        );
        assert!(
            should_use_clipboard(long_len, Some(proc_name)),
            "uzun metinde tüm hedefler pano kullanmalı: {proc_name}"
        );
        assert_eq!(
            paste_shortcut_for_process(Some(proc_name)),
            expected_shortcut,
            "yapıştırma kısayolu hatalı: {proc_name}"
        );
    }

    // Büyük/küçük harf duyarsız işlem eşleşmesi (Notepad & wt.exe)
    assert!(should_use_clipboard(12, Some("NOTEPAD.EXE")));
    assert_eq!(
        paste_shortcut_for_process(Some("wt.exe")),
        PasteShortcut::CtrlShiftV
    );
    assert_eq!(
        paste_shortcut_for_process(Some("WINDOWSTERMINAL.EXE")),
        PasteShortcut::CtrlShiftV
    );

    // Kısa metin SendInput dizisi: Türkçe harfler + satır sonu + sekme + emoji surrogate çifti
    let seq = build_short_text_sequence(4, "İğşü\n\t🐑", ' ');
    assert_eq!(&seq[0..4], &[InjectStep::Backspace; 4]);
    assert_eq!(seq[4], InjectStep::UnicodeUnit('İ' as u16));
    assert_eq!(seq[5], InjectStep::UnicodeUnit('ğ' as u16));
    assert_eq!(seq[6], InjectStep::UnicodeUnit('ş' as u16));
    assert_eq!(seq[7], InjectStep::UnicodeUnit('ü' as u16));
    assert_eq!(seq[8], InjectStep::Enter);
    assert_eq!(seq[9], InjectStep::Tab);
    // '🐑' UTF-16 surrogate çifti (2 birim) + boşluk tetiği = 3 adım
    assert_eq!(seq.len(), 13);
    assert_eq!(seq[12], InjectStep::UnicodeUnit(' ' as u16));

    // `{imleç}` konumu → gereken sayıda sol ok adımı
    let now = Local.with_ymd_and_hms(2026, 11, 26, 14, 30, 0).unwrap();
    let ctx = ResolveContext {
        now,
        clipboard_text: None,
        field_values: HashMap::new(),
        fold_dotted_i: false,
        phrase_lookup: Box::new(|_| None),
    };
    let resolved = resolve_template("Sayın {imleç},\nİyi çalışmalar.", &ctx);
    assert_eq!(resolved.text, "Sayın ,\nİyi çalışmalar.");
    assert_eq!(
        resolved.cursor_left_steps,
        ",\nİyi çalışmalar.".chars().count()
    );

    // Tetik karakteriyle birlikte toplam geri ok sayısı (+1)
    let left_steps = build_cursor_left_steps(resolved.cursor_left_steps + 1);
    assert_eq!(left_steps.len(), resolved.cursor_left_steps + 1);
    assert!(left_steps.iter().all(|s| *s == InjectStep::LeftArrow));

    // Yeniden seçim adımı CRLF'yi tek satır sonu sayar
    assert_eq!(build_reselect_left_count("Satır 1\r\nSatır 2"), 15);
}

#[test]
fn uat_app_rules_only_exclude_across_target_matrix() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(&dir.path().join("uat-rules.db")).unwrap();

    // 1. Genel ifade (Excel ve Windows Terminal'de hariç tutulmuş)
    let general = storage
        .create_phrase(&NewPhrase {
            abbr: "slm".to_string(),
            content: "Merhaba, iyi çalışmalar.".to_string(),
            content_html: None,
            description: "Genel selamlama".to_string(),
            folder_id: None,
            folder_ids: vec![],
            case_mode: CaseMode::KeepOriginal,
            shortcut: None,
            tags: vec!["genel".to_string()],
        })
        .unwrap();
    storage
        .set_phrase_app_rules(
            general.id,
            &[
                AppRule {
                    kind: AppRuleKind::Exclude,
                    process_name: "EXCEL.EXE".to_string(),
                    title_contains: None,
                },
                AppRule {
                    kind: AppRuleKind::Exclude,
                    process_name: "WindowsTerminal.exe".to_string(),
                    title_contains: None,
                },
            ],
        )
        .unwrap();

    // 2. Veritabanı kalıcılığını doğrula ve bellek içi eşleyiciyi besle
    let mut entries = storage.load_matcher_entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].app_rules.len(), 2);

    // Aynı kısaltmaya sahip programa özel (Only) ifadeler ekle: Word (küçük harf kuralıyla WINWORD.EXE eşleşmeli), Chrome (başlık), Slack
    let mut word_only = make_entry(101, "slm", "Sayın Yetkili,", CaseMode::KeepOriginal);
    word_only.app_rules.push(AppRule {
        kind: AppRuleKind::Only,
        process_name: "winword.exe".to_string(),
        title_contains: Some("Belge".to_string()),
    });

    let mut chrome_mail = make_entry(102, "slm", "İyi günler dilerim,", CaseMode::KeepOriginal);
    chrome_mail.app_rules.push(AppRule {
        kind: AppRuleKind::Only,
        process_name: "chrome.exe".to_string(),
        title_contains: Some("Web Posta".to_string()),
    });

    let mut slack_only = make_entry(103, "slm", "Selam ekip!", CaseMode::KeepOriginal);
    slack_only.app_rules.push(AppRule {
        kind: AppRuleKind::Only,
        process_name: "slack.exe".to_string(),
        title_contains: None,
    });

    entries.push(word_only);
    entries.push(chrome_mail);
    entries.push(slack_only);

    let mut matcher = Matcher::new(false);
    matcher.rebuild(entries);

    // 6 hedef uygulama matrisinin tamamını tek tek doğrula
    for &(proc_name, title, _, _) in TARGET_MATRIX {
        let ctx = ActiveContext {
            process_name: Some(proc_name.to_string()),
            window_title: Some(title.to_string()),
        };
        let matched = matcher.try_match("slm", ' ', &ctx);
        match proc_name {
            "WINWORD.EXE" => {
                assert_eq!(matched.expect("Word kuralı çalışmalı").phrase_id, 101);
            }
            "chrome.exe" => {
                assert_eq!(
                    matched.expect("Chrome başlık kuralı çalışmalı").phrase_id,
                    102
                );
            }
            "slack.exe" => {
                assert_eq!(matched.expect("Slack kuralı çalışmalı").phrase_id, 103);
            }
            "notepad.exe" => {
                assert_eq!(
                    matched.expect("Notepad genel ifadeyi almalı").phrase_id,
                    general.id
                );
            }
            "EXCEL.EXE" | "WindowsTerminal.exe" => {
                assert!(
                    matched.is_none(),
                    "Exclude kuralı nedeniyle genişlememeli: {proc_name}"
                );
            }
            other => panic!("Beklenmeyen matris girdisi: {other}"),
        }
    }

    // Chrome'da farklı sekme başlığı varsa Only eşleşmez, genel ifadeye düşer
    let chrome_other_tab = ActiveContext {
        process_name: Some("CHROME.EXE".to_string()),
        window_title: Some("Dokümantasyon - Google Chrome".to_string()),
    };
    assert_eq!(
        matcher
            .try_match("slm", ' ', &chrome_other_tab)
            .unwrap()
            .phrase_id,
        general.id
    );

    // Küresel dışlama ve balon gizleme kuralları
    let excluded = vec![
        "keepassxc.exe".to_string(),
        "windowsterminal.exe".to_string(),
    ];
    assert!(is_process_excluded(Some("WindowsTerminal.exe"), &excluded));
    assert!(!is_process_excluded(Some("WINWORD.EXE"), &excluded));
    assert!(is_bubble_hidden_process(Some("Photoshop.exe"), &excluded));
    assert!(is_bubble_hidden_process(Some("meetin.exe"), &excluded));
    assert!(is_bubble_hidden_process(
        Some("WindowsTerminal.exe"),
        &excluded
    ));
    assert!(!is_bubble_hidden_process(Some("notepad.exe"), &excluded));
}

#[test]
fn uat_turkish_case_modes_shift_f3_and_ignore_accents() {
    let phrase = "İstanbul Iğdır Çiçek Şirketi";

    // 1. Üç harf uyumu modu (MatchTyped, KeepOriginal, ExactMatch)
    assert_eq!(
        apply_case("mipy", phrase, CaseMode::MatchTyped),
        "istanbul ığdır çiçek şirketi"
    );
    assert_eq!(
        apply_case("Mipy", phrase, CaseMode::MatchTyped),
        "İstanbul ığdır çiçek şirketi"
    );
    assert_eq!(
        apply_case("MİPY", phrase, CaseMode::MatchTyped),
        "İSTANBUL IĞDIR ÇİÇEK ŞİRKETİ"
    );
    assert_eq!(apply_case("mİpY", phrase, CaseMode::MatchTyped), phrase);

    for typed in ["mipy", "Mipy", "MİPY"] {
        assert_eq!(apply_case(typed, phrase, CaseMode::KeepOriginal), phrase);
    }

    // Birleşik Unicode nokta (I + U+0307)
    assert_eq!(tr_lower("I\u{0307}STANBUL"), "istanbul");
    assert_eq!(tr_upper("istanbul ığdır"), "İSTANBUL IĞDIR");
    assert_eq!(tr_title("ığdır ili"), "Iğdır ili");

    // 2. Shift+F3 döngüsü: küçük → İlk harf büyük → TÜMÜ BÜYÜK → küçük
    let step1 = cycle_turkish_case("istanbul ığdır şelalesi");
    assert_eq!(step1, "İstanbul ığdır şelalesi");
    let step2 = cycle_turkish_case(&step1);
    assert_eq!(step2, "İSTANBUL IĞDIR ŞELALESİ");
    let step3 = cycle_turkish_case(&step2);
    assert_eq!(step3, "istanbul ığdır şelalesi");
    assert_eq!(cycle_turkish_case("Karışık İfade"), "karışık ifade");

    // 3. ExactMatch + Aksanları yoksay (fold_dotted_i)
    let mut matcher = Matcher::new(false);
    matcher.rebuild(vec![
        make_entry(1, "MİPY", "Metin ile Pratik Yazım", CaseMode::MatchTyped),
        make_entry(2, "ÇİÇEK", "Çiçek Sipariş Şablonu", CaseMode::KeepOriginal),
        make_entry(3, "ÖZEL", "Birebir Kısaltma", CaseMode::ExactMatch),
    ]);
    let ctx = ActiveContext::default();

    // Aksanları yoksay KAPALI: MIPY (noktasız I) MİPY ile eşleşmez
    assert!(matcher.try_match("MIPY", ' ', &ctx).is_none());
    assert!(matcher.try_match("cicek", ' ', &ctx).is_none());
    assert!(matcher.try_match("MİPY", ' ', &ctx).is_some());

    // ExactMatch: küçük harfle eşleşmez, tam eşleşmede çalışır
    assert!(matcher.try_match("özel", ' ', &ctx).is_none());
    assert_eq!(
        matcher.try_match("ÖZEL", ' ', &ctx).unwrap().expanded_text,
        "Birebir Kısaltma"
    );

    // Aksanları yoksay AÇIK: Türkçe aksanlı karakterler ASCII karşılıklarıyla eşleşir
    matcher.set_fold_dotted_i(true);
    assert_eq!(
        matcher.try_match("MIPY", ' ', &ctx).unwrap().expanded_text,
        "METİN İLE PRATİK YAZIM"
    );
    assert_eq!(
        matcher.try_match("cicek", ' ', &ctx).unwrap().expanded_text,
        "Çiçek Sipariş Şablonu"
    );
    assert_eq!(normalize_abbr("ŞİRKET", true), "sirket");
    assert_eq!(normalize_abbr("ÖĞÜN", true), "ogun");
    assert_eq!(normalize_abbr("IŞIK", true), "isik");
}

#[test]
fn uat_rich_html_to_cf_html_rtf_and_plain_text() {
    let html = "<p><strong>Önemli</strong> ve <em>hızlı</em> <u>not</u>: İğşüöç &amp; kontrol</p>\
                <ul><li>Birinci madde</li><li>İkinci madde</li></ul>\
                <ol><li>Adım bir</li><li>Adım iki</li></ol>\
                <p><a href=\"https://example.org/belge\">Bağlantıyı aç</a></p>";

    // 1. CF_HTML v0.9 başlık ofsetleri tam bayt eşleşmesi
    let cf_bytes = build_cf_html(html);
    assert_eq!(cf_bytes.last().copied(), Some(0));
    let cf_text = std::str::from_utf8(&cf_bytes[..cf_bytes.len() - 1]).unwrap();
    assert!(cf_text.starts_with("Version:0.9\r\n"));

    let parse_offset = |prefix: &str| -> usize {
        cf_text
            .lines()
            .find(|l| l.starts_with(prefix))
            .unwrap()
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    };
    let start_html = parse_offset("StartHTML:");
    let end_html = parse_offset("EndHTML:");
    let start_frag = parse_offset("StartFragment:");
    let end_frag = parse_offset("EndFragment:");

    assert!(start_html < start_frag);
    assert!(end_frag < end_html);
    assert_eq!(end_html, cf_text.len());
    assert_eq!(&cf_text.as_bytes()[start_frag..end_frag], html.as_bytes());

    // 2. Word/Outlook uyumlu RTF dönüşümü (cp1254 + Unicode kaçışları + liste + bağlantı)
    let rtf_bytes = html_to_rtf(html);
    assert_eq!(rtf_bytes.last().copied(), Some(0));
    let rtf = std::str::from_utf8(&rtf_bytes[..rtf_bytes.len() - 1]).unwrap();
    assert!(rtf.starts_with("{\\rtf1\\ansi\\ansicpg1254"));
    assert!(rtf.ends_with('}'));
    assert!(rtf.contains("{\\b \\u214?nemli}")); // Önemli
    assert!(rtf.contains("{\\i h\\u305?zl\\u305?}")); // hızlı
    assert!(rtf.contains("{\\ul not}"));
    assert!(rtf.contains("\\bullet  Birinci madde\\par "));
    assert!(rtf.contains("\\bullet  \\u304?kinci madde\\par "));
    assert!(rtf.contains("1. Ad\\u305?m bir\\par "));
    assert!(rtf.contains("2. Ad\\u305?m iki\\par "));
    assert!(rtf.contains("{\\ul Ba\\u287?lant\\u305?y\\u305? a\\u231?}"));

    // 3. Düz metin çıkarımı (Notepad / Terminal gibi hedefler için)
    let plain = html_to_plain_text(html);
    assert!(plain.contains("Önemli ve hızlı not: İğşüöç & kontrol"));
    assert!(plain.contains("• Birinci madde\n• İkinci madde"));
    assert!(plain.contains("1. Adım bir\n2. Adım iki"));
    assert!(plain.contains("Bağlantıyı aç"));
}

#[test]
fn uat_selection_filter_and_password_safety() {
    // Boşluk / yalnızca kontrol karakteri elenir
    assert_eq!(clean_selected_text(None), None);
    assert_eq!(clean_selected_text(Some(String::new())), None);
    assert_eq!(clean_selected_text(Some("   \t\r\n  ".to_string())), None);
    assert_eq!(
        clean_selected_text(Some("Türkçe Seçili Metin".to_string())),
        Some("Türkçe Seçili Metin".to_string())
    );

    // Parola alanında (IsPassword = true) ASLA metin dönmez ve Ctrl+C yedeği tetiklenmez (§7.4)
    let fallback_called = AtomicBool::new(false);
    let pw_result = resolve_selection_outcome(SelectionSourceOutcome::PasswordField, || {
        fallback_called.store(true, Ordering::SeqCst);
        Some("gizli-parola-123".to_string())
    });
    assert_eq!(pw_result, None);
    assert!(
        !fallback_called.load(Ordering::SeqCst),
        "Parola alanında Ctrl+C yedek yolu kesinlikle çağrılmamalı"
    );

    // UIA TextPattern başarılıysa yedek çağrılmaz
    let text_fallback_called = AtomicBool::new(false);
    let uia_ok = resolve_selection_outcome(
        SelectionSourceOutcome::Text("Seçili rapor özeti".to_string()),
        || {
            text_fallback_called.store(true, Ordering::SeqCst);
            Some("pano".to_string())
        },
    );
    assert_eq!(uia_ok, Some("Seçili rapor özeti".to_string()));
    assert!(!text_fallback_called.load(Ordering::SeqCst));

    // Desteklenmeyen denetimde yedek yol çalışır ve boşlukları yine süzer
    let fb_blank = resolve_selection_outcome(SelectionSourceOutcome::Unsupported, || {
        Some("  \n ".to_string())
    });
    assert_eq!(fb_blank, None);

    let fb_ok = resolve_selection_outcome(SelectionSourceOutcome::Unsupported, || {
        Some("Pano yedeğinden okunan metin".to_string())
    });
    assert_eq!(fb_ok, Some("Pano yedeğinden okunan metin".to_string()));
}

#[test]
fn uat_mee_zip_rich_media_round_trip_and_integrity() {
    const SAMPLE_PNG_B64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
    let svg_b64 = encode_base64(
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\"/></svg>",
    );
    let original_html = format!(
        "<p><strong>İmza Bloğu</strong></p><img src=\"data:image/png;base64,{SAMPLE_PNG_B64}\" alt=\"simge\"><img src=\"data:image/svg+xml;base64,{svg_b64}\" alt=\"vektor\">"
    );

    let folders = vec![FolderRecord {
        id: 7,
        parent_id: None,
        name: "Kurumsal Şablonlar".to_string(),
        color: Some("c3".to_string()),
        sort_order: 1,
    }];
    let phrases = vec![PhraseRecord {
        id: 1,
        folder_id: Some(7),
        folder_ids: vec![7],
        abbr: "imza1".to_string(),
        content: "İmza Bloğu".to_string(),
        content_html: Some(original_html.clone()),
        description: "Görselli imza".to_string(),
        case_mode: CaseMode::KeepOriginal,
        shortcut: None,
        enabled: true,
        use_count: 3,
        last_used_at: None,
        created_at: String::new(),
        updated_at: String::new(),
        deleted_at: None,
        tags: vec!["imza".to_string(), "medya".to_string()],
    }];

    // 1. ZIP .mee dışa aktar ve imzayı doğrula
    let zip_bytes = export_mee_archive(&phrases, &folders, "1.2.0").unwrap();
    assert!(is_zip_signature(&zip_bytes));

    // 2. Doğrudan import_mee_archive ve parse_import_file ile kayıpsız geri yükleme
    let direct_items = import_mee_archive(&zip_bytes).unwrap();
    assert_eq!(direct_items.len(), 1);
    assert_eq!(
        direct_items[0].content_html.as_deref(),
        Some(original_html.as_str())
    );
    assert_eq!(
        direct_items[0].folder.as_deref(),
        Some("Kurumsal Şablonlar")
    );
    assert_eq!(
        direct_items[0].tags,
        vec!["imza".to_string(), "medya".to_string()]
    );

    let parsed = parse_import_file("yedek.mee", &zip_bytes).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].content_html.as_deref(),
        Some(original_html.as_str())
    );

    // 3. Temiz veritabanına içe aktarım
    let dir = TempDir::new().unwrap();
    let dst_db = Storage::open(&dir.path().join("imported.db")).unwrap();
    let summary = import_items(&dst_db, &parsed, ConflictStrategy::Overwrite, false).unwrap();
    assert_eq!(summary.imported, 1);
    let stored = dst_db.list_phrases(false).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].content_html.as_deref(),
        Some(original_html.as_str())
    );

    // 4. Geriye dönük düz JSON .mee / .meetin desteği
    let legacy_json = export_meetin_json(&phrases, &folders).unwrap();
    let legacy_parsed = parse_import_file("eski.mee", legacy_json.as_bytes()).unwrap();
    assert_eq!(legacy_parsed.len(), 1);
    assert_eq!(legacy_parsed[0].abbr, "imza1");

    // 5. Bozuk CRC32 ve kesilmiş ZIP güvenlik reddi
    let mut tampered = zip_bytes.clone();
    // Local file payload içinde bir baytı boz (manifest.json içeriği)
    let corrupt_idx = 60.min(tampered.len() - 1);
    tampered[corrupt_idx] ^= 0xFF;
    let err = import_mee_archive(&tampered).unwrap_err();
    assert!(
        matches!(
            err,
            MeeArchiveError::CrcMismatch { .. } | MeeArchiveError::InvalidZip(_)
        ),
        "Bozuk arşiv yakalanmalı, alınan: {err}"
    );

    let truncated = &zip_bytes[..24];
    let trunc_err = parse_import_file("bozuk.mee", truncated).unwrap_err();
    assert!(!trunc_err.to_string().is_empty());
}

#[test]
fn uat_end_to_end_typing_variables_and_suggestions() {
    let mut matcher = Matcher::new(false);
    matcher.rebuild(vec![
        make_entry(
            1,
            "rpr",
            "Rapor ({tarih} {saat}): {alan:Konu} - {ifade:İMZA} [{imleç}]",
            CaseMode::KeepOriginal,
        ),
        make_entry(2, "rpr2", "İkinci rapor şablonu", CaseMode::KeepOriginal),
        make_entry(3, "İMZA", "Saygılarımla", CaseMode::KeepOriginal),
    ]);

    let word_ctx = ActiveContext {
        process_name: Some("WINWORD.EXE".to_string()),
        window_title: Some("Haftalık Özet - Word".to_string()),
    };

    // Yazarken önek önerisi (2+ harf)
    let mut buf = KeyBuffer::new();
    for ch in "Yeni rp".chars() {
        buf.push(ch);
    }
    let suggestions = matcher.suggest_prefix(&buf.as_string(), &word_ctx, 3);
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].abbr, "rpr");

    // Hatalı harf → Backspace → doğru harf → tetik
    buf.push('x');
    buf.backspace();
    buf.push('r');

    let matched = matcher
        .try_match(&buf.as_string(), ' ', &word_ctx)
        .expect("kısaltma eşleşmeli");
    assert_eq!(matched.phrase_id, 1);

    let fields = extract_form_fields(&matched.expanded_text);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "Konu");

    let now = Local.with_ymd_and_hms(2026, 11, 26, 16, 45, 0).unwrap();
    let resolve_ctx = ResolveContext {
        now,
        clipboard_text: None,
        field_values: HashMap::from([("Konu".to_string(), "Kararlılık".to_string())]),
        fold_dotted_i: false,
        phrase_lookup: Box::new(|abbr| {
            if abbr == "İMZA" {
                Some("Saygılarımla".to_string())
            } else {
                None
            }
        }),
    };
    let resolved = resolve_template(&matched.expanded_text, &resolve_ctx);
    assert_eq!(
        resolved.text,
        "Rapor (26.11.2026 16:45): Kararlılık - Saygılarımla []"
    );
    assert_eq!(resolved.cursor_left_steps, 1);
}

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < input.len() {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
