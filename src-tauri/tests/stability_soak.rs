//! Bellek Sızıntısı ve Yüksek Hacimli Kararlılık Stres Testi (Faz E).
//!
//! Uzun süreli (24 saat+) masaüstü kullanımını deterministik olarak simüle eder:
//! 1. 120.000+ karakterlik tuş arabelleği ve 10.000 ifadeli eşleme döngüsü:
//!    - `KeyBuffer` uzunluğu her adımda ≤ 64 karakter kalır,
//!    - 10.000 ifadede arama süresi < 5 ms eşiğinin altında kalır.
//! 2. Yüzlerce ardışık SQLite CRUD, sürüm geçmişi, pano geçmişi (limit kırpma)
//!    ve `VACUUM INTO` yedekleme/rotasyon döngüsü.
//! 3. Windows `K32GetProcessMemoryInfo` ile Working Set artışı < 15 MB doğrulaması.

use std::fs;
use std::time::{Duration, Instant};

use chrono::{Days, NaiveDate};
use tempfile::TempDir;

use meetin_lib::db::{NewPhrase, Storage, UpdatePhrase};
use meetin_lib::hook::buffer::{KeyBuffer, MAX_BUFFER_CHARS};
use meetin_lib::matcher::case::CaseMode;
use meetin_lib::matcher::{ActiveContext, AppRule, AppRuleKind, Matcher, PhraseEntry};

const MAX_MEMORY_GROWTH_BYTES: usize = 15 * 1024 * 1024;

#[cfg(windows)]
fn current_working_set_bytes() -> usize {
    use std::mem::size_of;
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let mut pmc = PROCESS_MEMORY_COUNTERS::default();
        let ok = K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut pmc,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        assert!(ok.as_bool(), "K32GetProcessMemoryInfo başarısız oldu");
        pmc.WorkingSetSize
    }
}

#[cfg(not(windows))]
fn current_working_set_bytes() -> usize {
    0
}

fn build_large_matcher(count: usize) -> Matcher {
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let mut app_rules = Vec::new();
        if i % 25 == 0 {
            app_rules.push(AppRule {
                kind: AppRuleKind::Exclude,
                process_name: "WindowsTerminal.exe".to_string(),
                title_contains: None,
            });
        }
        entries.push(PhraseEntry {
            id: (i + 1) as i64,
            abbr: format!("kıs{i}"),
            content: format!("Kararlılık testi genişletilmiş metin kaydı #{i} — Türkçe İğşüöç"),
            content_html: None,
            case_mode: match i % 3 {
                0 => CaseMode::MatchTyped,
                1 => CaseMode::KeepOriginal,
                _ => CaseMode::ExactMatch,
            },
            app_rules,
            enabled: true,
        });
    }
    let mut matcher = Matcher::new(false);
    matcher.rebuild(entries);
    matcher
}

#[test]
fn soak_key_buffer_120k_chars_and_10000_phrase_matcher() {
    let matcher = build_large_matcher(10_000);
    assert_eq!(matcher.len(), 10_000);

    let contexts = [
        ActiveContext {
            process_name: Some("WINWORD.EXE".to_string()),
            window_title: Some("Belge - Word".to_string()),
        },
        ActiveContext {
            process_name: Some("EXCEL.EXE".to_string()),
            window_title: Some("Tablo - Excel".to_string()),
        },
        ActiveContext {
            process_name: Some("chrome.exe".to_string()),
            window_title: Some("Web - Chrome".to_string()),
        },
        ActiveContext {
            process_name: Some("slack.exe".to_string()),
            window_title: Some("Ekip - Slack".to_string()),
        },
        ActiveContext {
            process_name: Some("notepad.exe".to_string()),
            window_title: Some("Not Defteri".to_string()),
        },
        ActiveContext {
            process_name: Some("WindowsTerminal.exe".to_string()),
            window_title: Some("Terminal".to_string()),
        },
    ];

    // Isınma turu (bellek tabanını kararlı hâle getir)
    let mut buf = KeyBuffer::new();
    for ch in "ısınma kıs9999".chars() {
        buf.push(ch);
    }
    let warm_hit = matcher.try_match(&buf.as_string(), ' ', &contexts[0]);
    assert!(warm_hit.is_some());
    buf.clear();

    let ws_before = current_working_set_bytes();

    // 120.000+ karakterlik uzun süreli yazım + silme + pencere geçişi + eşleme döngüsü
    let stream_chars: Vec<char> = "Merhaba dünya! İğşüöç kıs4242 test kıs9999 deneme, "
        .chars()
        .collect();
    let total_chars: usize = 120_000;
    let mut match_hits: usize = 0;

    for i in 0..total_chars {
        let ch = stream_chars[i % stream_chars.len()];
        if matches!(ch, ' ' | ',' | '!' | '.') {
            let ctx = &contexts[(i / 64) % contexts.len()];
            let snap = buf.as_string();
            if matcher.try_match(&snap, ch, ctx).is_some() {
                match_hits += 1;
            }
            buf.push(ch);
        } else {
            buf.push(ch);
        }

        // Periyodik yazım hatası silme (Backspace) ve pencere değişimi temizliği
        if i % 97 == 0 {
            buf.backspace();
        }
        if i % 1024 == 0 {
            buf.clear();
        }

        assert!(
            buf.len() <= MAX_BUFFER_CHARS,
            "Tuş arabelleği 64 karakter sınırını aştı: {}",
            buf.len()
        );
    }
    assert!(
        match_hits > 100,
        "Stres döngüsünde yeterli kısaltma eşleşmesi üretilmedi: {match_hits}"
    );

    // 10.000 ifadede arama gecikmesi doğrulaması (< 5 ms)
    let mut timings = Vec::with_capacity(200);
    for q in 0..200 {
        let idx = (q * 49) % 10_000;
        let query = format!("ön metin kıs{idx}");
        let start = Instant::now();
        let res = matcher.try_match(&query, ' ', &contexts[0]);
        let elapsed = start.elapsed();
        assert!(res.is_some(), "kıs{idx} eşleşmeli");
        timings.push(elapsed);
    }
    timings.sort();
    let p95 = timings[(timings.len() * 95) / 100];
    let avg_nanos: u128 =
        timings.iter().map(Duration::as_nanos).sum::<u128>() / timings.len() as u128;

    assert!(
        p95 < Duration::from_millis(5),
        "10.000 ifadede p95 arama süresi 5 ms sınırını aştı: {p95:?}"
    );
    assert!(
        avg_nanos < Duration::from_millis(2).as_nanos(),
        "10.000 ifadede ortalama arama süresi çok yüksek: {} ns",
        avg_nanos
    );

    drop(timings);
    let ws_after = current_working_set_bytes();
    let growth = ws_after.saturating_sub(ws_before);
    assert!(
        growth < MAX_MEMORY_GROWTH_BYTES,
        "120k tuş/eşleme döngüsünde Working Set artışı 15 MB sınırını aştı: {:.2} MB (önce: {:.2} MB, sonra: {:.2} MB)",
        growth as f64 / (1024.0 * 1024.0),
        ws_before as f64 / (1024.0 * 1024.0),
        ws_after as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn soak_sqlite_crud_versions_clipboard_and_vacuum_backups() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("soak-meetin.db");
    let backups_dir = dir.path().join("backups");
    let storage = Storage::open(&db_path).unwrap();

    // Isınma: SQLite sayfa önbelleğini ve WAL dosyalarını başlat
    let folder = storage
        .create_folder("Kararlılık", None, Some("c1"))
        .unwrap();
    let warm = storage
        .create_phrase(&NewPhrase {
            abbr: "isın0".to_string(),
            content: "Isınma metni".to_string(),
            content_html: Some("<p><b>Isınma</b> metni</p>".to_string()),
            description: "Isınma".to_string(),
            folder_id: Some(folder.id),
            folder_ids: vec![folder.id],
            case_mode: CaseMode::MatchTyped,
            shortcut: None,
            tags: vec!["soak".to_string()],
        })
        .unwrap();
    storage.record_usage(warm.id, Some("WINWORD.EXE")).unwrap();
    storage
        .push_clipboard_history("ısınma pano", None, Some("notepad.exe"), 25)
        .unwrap();
    let base_day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    storage
        .create_backup_for_date(&backups_dir, base_day, 15)
        .unwrap();

    let ws_before = current_working_set_bytes();

    // 1. 250 ifade ekleme, 250 güncelleme (sürüm geçmişi), 50 sürüm geri alma,
    //    80 çöp kutusu + geri yükleme ve 80 kalıcı silme
    let mut created_ids = Vec::with_capacity(250);
    for i in 0..250usize {
        let rec = storage
            .create_phrase(&NewPhrase {
                abbr: format!("soak{i}"),
                content: format!("İlk içerik #{i} — Türkçe İğşüöç"),
                content_html: Some(format!("<p><strong>İçerik</strong> #{i}</p>")),
                description: format!("Açıklama #{i}"),
                folder_id: Some(folder.id),
                folder_ids: vec![folder.id],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec!["kararlılık".to_string(), format!("grup{}", i % 5)],
            })
            .unwrap();
        created_ids.push(rec.id);
    }

    for (idx, &id) in created_ids.iter().enumerate() {
        storage
            .update_phrase(&UpdatePhrase {
                id,
                abbr: format!("soak{idx}"),
                content: format!("Güncellenmiş içerik #{idx} — sürüm 2"),
                content_html: Some(format!("<p><em>Güncel</em> #{idx}</p>")),
                description: format!("Güncel açıklama #{idx}"),
                folder_id: Some(folder.id),
                folder_ids: vec![folder.id],
                case_mode: CaseMode::KeepOriginal,
                shortcut: None,
                enabled: true,
                tags: vec!["kararlılık".to_string(), "v2".to_string()],
            })
            .unwrap();

        if idx % 5 == 0 {
            let versions = storage.list_phrase_versions(id).unwrap();
            assert!(!versions.is_empty(), "Sürüm geçmişi oluşmalı");
            storage.restore_phrase_version(id, versions[0].id).unwrap();
        }
        storage.record_usage(id, Some("chrome.exe")).unwrap();
    }

    for &id in created_ids.iter().take(80) {
        storage.soft_delete_phrase(id).unwrap();
        storage.restore_phrase(id).unwrap();
    }
    for &id in created_ids.iter().skip(170) {
        storage.hard_delete_phrase(id).unwrap();
    }

    // Eşleyici kayıtlarını yeniden yükle (1 ısınma + 170 kalan = 171 aktif ifade)
    let loaded = storage.load_matcher_entries().unwrap();
    assert_eq!(loaded.len(), 171);

    // 2. 400 ardışık pano geçmişi kaydı — limit (25) kırpma doğrulaması
    let clip_limit = 25usize;
    for i in 0..400usize {
        let inserted = storage
            .push_clipboard_history(
                &format!("Pano kaydı #{i} — Örnek Türkçe metin"),
                None,
                Some(if i % 2 == 0 {
                    "WINWORD.EXE"
                } else {
                    "slack.exe"
                }),
                clip_limit,
            )
            .unwrap();
        assert!(inserted);
        if i % 50 == 49 {
            let hist = storage.list_clipboard_history(200).unwrap();
            assert!(
                hist.len() <= clip_limit,
                "Pano geçmişi limiti aşıldı: {}",
                hist.len()
            );
        }
    }
    let final_clip = storage.list_clipboard_history(200).unwrap();
    assert_eq!(final_clip.len(), clip_limit);
    assert!(final_clip[0].content.contains("#399"));

    // 3. 35 ardışık `VACUUM INTO` yedeği ve 15 dosya rotasyonu
    let max_backups = 15usize;
    let mut latest_backup = None;
    for day_offset in 1..=35u64 {
        let day = base_day + Days::new(day_offset);
        let path = storage
            .create_backup_for_date(&backups_dir, day, max_backups)
            .unwrap();
        latest_backup = Some(path);
    }

    let backup_files: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".db"))
        .collect();
    assert_eq!(
        backup_files.len(),
        max_backups,
        "Yedek rotasyonu tam {max_backups} dosya bırakmalı"
    );

    // Son yedeğin tutarlı ve açılabilir olduğunu doğrula
    let latest_path = latest_backup.unwrap();
    let reopened = Storage::open(&latest_path).unwrap();
    assert_eq!(reopened.list_phrases(false).unwrap().len(), 171);
    drop(reopened);
    drop(loaded);
    drop(final_clip);
    drop(created_ids);

    // 4. Windows Working Set artışı < 15 MB
    let ws_after = current_working_set_bytes();
    let growth = ws_after.saturating_sub(ws_before);
    assert!(
        growth < MAX_MEMORY_GROWTH_BYTES,
        "SQLite CRUD + yedekleme döngüsünde Working Set artışı 15 MB sınırını aştı: {:.2} MB (önce: {:.2} MB, sonra: {:.2} MB)",
        growth as f64 / (1024.0 * 1024.0),
        ws_before as f64 / (1024.0 * 1024.0),
        ws_after as f64 / (1024.0 * 1024.0)
    );
}
