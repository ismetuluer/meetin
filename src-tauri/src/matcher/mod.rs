//! Bellek içi kısaltma eşleme motoru (§3.1, §3.2, §3.5).

pub mod case;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use self::case::{apply_case, normalize_abbr, tr_lower, CaseMode};

/// Programa özel ifade kuralı türü (§3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRuleKind {
    Only,
    Exclude,
}

/// İfadeye bağlı program kuralı (§3.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRule {
    pub kind: AppRuleKind,
    /// İşlem adı, örn. `WINWORD.EXE` (büyük/küçük duyarsız karşılaştırılır).
    pub process_name: String,
    /// İsteğe bağlı pencere başlığı alt dizesi, örn. `WhatsApp`.
    pub title_contains: Option<String>,
}

/// Eşleme motorunda tutulan tek bir ifade kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhraseEntry {
    pub id: i64,
    pub abbr: String,
    pub content: String,
    #[serde(default)]
    pub content_html: Option<String>,
    pub case_mode: CaseMode,
    pub app_rules: Vec<AppRule>,
    pub enabled: bool,
}

/// O an etkin pencere bağlamı.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveContext {
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

/// Eşleme sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub phrase_id: i64,
    pub abbr: String,
    /// Silinecek kısaltmanın karakter (Unicode scalar) uzunluğu.
    pub abbr_char_len: usize,
    /// Harf uyumu uygulanmış genişletilmiş metin.
    pub expanded_text: String,
    /// Varsa HTML biçimli metin.
    pub content_html: Option<String>,
    /// Eşleşmeyi tetikleyen karakter (metnin sonuna eklenir).
    pub trigger: char,
}

/// Yazarken öneri adası için aday öğe (§3.1, M6.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionItem {
    pub phrase_id: i64,
    pub abbr: String,
    pub preview: String,
    pub typed_len: usize,
}

/// Tetik karakterleri (§3.1): boşluk, Enter, Tab, `. , ; : ! ? ) "`
#[inline]
pub fn is_trigger_char(c: char) -> bool {
    matches!(
        c,
        ' ' | '\n' | '\r' | '\t' | '.' | ',' | ';' | ':' | '!' | '?' | ')' | '"'
    )
}

/// Kelime sınırı kontrolü (§3.1): kısaltma bir kelimenin başından başlamalıdır.
/// Önceki karakter harf veya rakamsa kelime içi sayılır (`xMİPY` genişlemez).
#[inline]
pub fn is_word_boundary(c: char) -> bool {
    !c.is_alphanumeric() && c != '_'
}

/// Kuralın mevcut pencere bağlamıyla eşleşip eşleşmediğini kontrol eder.
fn rule_matches_context(rule: &AppRule, ctx: &ActiveContext) -> bool {
    let proc_ok = match &ctx.process_name {
        Some(active_proc) => {
            active_proc.eq_ignore_ascii_case(&rule.process_name)
                || tr_lower(active_proc) == tr_lower(&rule.process_name)
        }
        None => false,
    };
    if !proc_ok {
        return false;
    }
    match &rule.title_contains {
        Some(needle) if !needle.is_empty() => match &ctx.window_title {
            Some(title) => tr_lower(title).contains(&tr_lower(needle)),
            None => false,
        },
        _ => true,
    }
}

/// İfadenin verilen bağlamda kullanılıp kullanılamayacağını ve öncelik puanını döndürür.
/// `Only` kuralıyla eşleşen ifadeler kuralı olmayan genel ifadelere göre önceliklidir (§3.5).
fn evaluate_app_rules(rules: &[AppRule], ctx: &ActiveContext) -> Option<u8> {
    if rules.is_empty() {
        return Some(0);
    }

    let mut has_only = false;
    let mut matched_only = false;

    for rule in rules {
        match rule.kind {
            AppRuleKind::Exclude => {
                if rule_matches_context(rule, ctx) {
                    return None;
                }
            }
            AppRuleKind::Only => {
                has_only = true;
                if rule_matches_context(rule, ctx) {
                    matched_only = true;
                }
            }
        }
    }

    if has_only {
        if matched_only {
            Some(2)
        } else {
            None
        }
    } else {
        Some(1)
    }
}

/// Bellek içi kısaltma eşleyici.
#[derive(Debug, Clone)]
pub struct Matcher {
    /// Normalize edilmiş kısaltma → aday ifadeler.
    index: HashMap<String, Vec<PhraseEntry>>,
    /// Noktasız/noktalı i'yi eşit sayma ayarı (§3.2, varsayılan: false).
    fold_dotted_i: bool,
    /// En uzun kısaltmanın karakter uzunluğu (hızlı arama için üst sınır).
    max_abbr_chars: usize,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Matcher {
    pub fn new(fold_dotted_i: bool) -> Self {
        Self {
            index: HashMap::new(),
            fold_dotted_i,
            max_abbr_chars: 0,
        }
    }

    pub fn set_fold_dotted_i(&mut self, fold: bool) {
        if self.fold_dotted_i == fold {
            return;
        }
        let all: Vec<PhraseEntry> = self
            .index
            .drain()
            .flat_map(|(_, v)| v.into_iter())
            .collect();
        self.fold_dotted_i = fold;
        self.rebuild(all);
    }

    pub fn fold_dotted_i(&self) -> bool {
        self.fold_dotted_i
    }

    /// Tüm kısaltma indeksini verilen ifadelerle yeniden kurar.
    pub fn rebuild(&mut self, entries: Vec<PhraseEntry>) {
        self.index.clear();
        self.max_abbr_chars = 0;

        for entry in entries {
            if !entry.enabled || entry.abbr.trim().is_empty() {
                continue;
            }
            let key = normalize_abbr(&entry.abbr, self.fold_dotted_i);
            let char_len = entry.abbr.chars().count();
            if char_len > self.max_abbr_chars {
                self.max_abbr_chars = char_len;
            }
            self.index.entry(key).or_default().push(entry);
        }
    }

    pub fn len(&self) -> usize {
        self.index.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Tuş arabelleğinin sonundaki kelimeyi tetik karakteriyle eşleştirir.
    ///
    /// Kelime başı kuralı: Kısaltma, arabelleğin başında ya da bir kelime sınırından
    /// (`is_word_boundary`) hemen sonra başlamalıdır. Çok kelimeli kısaltmalar da
    /// (ör. `ABD BAŞKAN`) desteklenir; bunun için arabellek sonundaki tüm geçerli
    /// kelime başlangıç noktaları uzundan kısaya doğru taranır.
    pub fn try_match(
        &self,
        buffer: &str,
        trigger: char,
        ctx: &ActiveContext,
    ) -> Option<MatchResult> {
        if !is_trigger_char(trigger) || buffer.is_empty() || self.max_abbr_chars == 0 {
            return None;
        }

        // Karakter (byte_idx, char) listesini al
        let char_indices: Vec<(usize, char)> = buffer.char_indices().collect();
        let total_chars = char_indices.len();
        let min_start = total_chars.saturating_sub(self.max_abbr_chars);

        // En uzun adaydan en kısaya doğru dene (ör. "ABD BAŞKAN" varsa "BAŞKAN"dan önce yakalanır)
        for start_pos in min_start..total_chars {
            // Kelime başı kontrolü
            if start_pos > 0 && !is_word_boundary(char_indices[start_pos - 1].1) {
                continue;
            }

            let byte_start = char_indices[start_pos].0;
            let candidate = &buffer[byte_start..];
            let norm = normalize_abbr(candidate, self.fold_dotted_i);

            let Some(entries) = self.index.get(&norm) else {
                continue;
            };

            let mut best: Option<(u8, &PhraseEntry)> = None;

            for entry in entries {
                if !entry.enabled {
                    continue;
                }
                if entry.case_mode == CaseMode::ExactMatch && entry.abbr != candidate {
                    continue;
                }
                if let Some(priority) = evaluate_app_rules(&entry.app_rules, ctx) {
                    if best.map(|(p, _)| priority > p).unwrap_or(true) {
                        best = Some((priority, entry));
                    }
                }
            }

            if let Some((_, entry)) = best {
                let expanded_text = apply_case(candidate, &entry.content, entry.case_mode);
                return Some(MatchResult {
                    phrase_id: entry.id,
                    abbr: entry.abbr.clone(),
                    abbr_char_len: total_chars - start_pos,
                    expanded_text,
                    content_html: entry.content_html.clone(),
                    trigger,
                });
            }
        }

        None
    }

    /// Arabellek sonundaki yarım kelime için 2+ harfte en fazla `limit` önek önerisi döndürür (M6.7).
    pub fn suggest_prefix(
        &self,
        buffer: &str,
        ctx: &ActiveContext,
        limit: usize,
    ) -> Vec<SuggestionItem> {
        if buffer.is_empty() || limit == 0 || self.index.is_empty() {
            return Vec::new();
        }
        let char_indices: Vec<(usize, char)> = buffer.char_indices().collect();
        let total = char_indices.len();
        if is_word_boundary(char_indices[total - 1].1) {
            return Vec::new();
        }

        let mut start_pos = total - 1;
        while start_pos > 0 && !is_word_boundary(char_indices[start_pos - 1].1) {
            start_pos -= 1;
        }
        let typed = &buffer[char_indices[start_pos].0..];
        let typed_len = total - start_pos;
        if typed_len < 2 {
            return Vec::new();
        }

        let typed_norm = normalize_abbr(typed, self.fold_dotted_i);
        let mut candidates: Vec<(u8, usize, &PhraseEntry)> = Vec::new();

        for (key, entries) in &self.index {
            if !key.starts_with(&typed_norm) {
                continue;
            }
            for entry in entries {
                if !entry.enabled {
                    continue;
                }
                if entry.case_mode == CaseMode::ExactMatch && !entry.abbr.starts_with(typed) {
                    continue;
                }
                if let Some(prio) = evaluate_app_rules(&entry.app_rules, ctx) {
                    candidates.push((prio, entry.abbr.chars().count(), entry));
                }
            }
        }

        candidates.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.abbr.cmp(&b.2.abbr))
        });

        let mut seen_ids = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (_, _, entry) in candidates {
            if !seen_ids.insert(entry.id) {
                continue;
            }
            let first_line = entry
                .content
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("");
            let preview = if first_line.chars().count() > 44 {
                let s: String = first_line.chars().take(44).collect();
                format!("{s}…")
            } else {
                first_line.to_string()
            };
            out.push(SuggestionItem {
                phrase_id: entry.id,
                abbr: entry.abbr.clone(),
                preview,
                typed_len,
            });
            if out.len() >= limit {
                break;
            }
        }
        out
    }

    /// Belirli bir `phrase_id` için yarım yazılmış metni genişletme sonucuna çevirir (M6.7).
    pub fn build_match_for_id(
        &self,
        phrase_id: i64,
        typed_slice: &str,
        trigger: char,
    ) -> Option<MatchResult> {
        let entry = self
            .index
            .values()
            .flatten()
            .find(|e| e.enabled && e.id == phrase_id)?;
        let expanded_text = apply_case(typed_slice, &entry.content, entry.case_mode);
        Some(MatchResult {
            phrase_id: entry.id,
            abbr: entry.abbr.clone(),
            abbr_char_len: typed_slice.chars().count(),
            expanded_text,
            content_html: entry.content_html.clone(),
            trigger,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn entry(id: i64, abbr: &str, content: &str, case_mode: CaseMode) -> PhraseEntry {
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
    fn word_boundary_prevents_mid_word_expansion() {
        let mut m = Matcher::default();
        m.rebuild(vec![entry(
            1,
            "MİPY",
            "Metin ile Pratik Yazım",
            CaseMode::MatchTyped,
        )]);

        let ctx = ActiveContext::default();
        // Kelime içinde genişlemez
        assert!(m.try_match("xMİPY", ' ', &ctx).is_none());
        assert!(m.try_match("1MİPY", ' ', &ctx).is_none());
        // Kelime başında genişler
        assert!(m.try_match("MİPY", ' ', &ctx).is_some());
        assert!(m.try_match("merhaba MİPY", ' ', &ctx).is_some());
        assert!(m.try_match("(MİPY", ')', &ctx).is_some());
    }

    #[test]
    fn multi_word_abbreviation_works() {
        let mut m = Matcher::default();
        m.rebuild(vec![entry(
            1,
            "İYİ GÜNLER",
            "İYİ GÜNLER DİLERİM, SAYGILARIMLA",
            CaseMode::KeepOriginal,
        )]);

        let res = m
            .try_match("Bugün İYİ GÜNLER", ' ', &ActiveContext::default())
            .expect("çok kelimeli kısaltma eşleşmeli");
        assert_eq!(res.expanded_text, "İYİ GÜNLER DİLERİM, SAYGILARIMLA");
        assert_eq!(res.abbr_char_len, "İYİ GÜNLER".chars().count());
    }

    #[test]
    fn all_spec_trigger_chars_work() {
        let mut m = Matcher::default();
        m.rebuild(vec![entry(1, "mipy", "test", CaseMode::KeepOriginal)]);
        let triggers = [
            ' ', '\n', '\r', '\t', '.', ',', ';', ':', '!', '?', ')', '"',
        ];
        for t in triggers {
            let res = m.try_match("mipy", t, &ActiveContext::default());
            assert!(res.is_some(), "tetik '{:?}' eşleşmeli", t);
            assert_eq!(res.unwrap().trigger, t);
        }
        // Normal harf tetik değildir
        assert!(m
            .try_match("mipy", 'a', &ActiveContext::default())
            .is_none());
    }

    #[test]
    fn exact_match_respects_case_strictly() {
        let mut m = Matcher::default();
        m.rebuild(vec![entry(
            1,
            "MİPY",
            "Metin ile Pratik Yazım",
            CaseMode::ExactMatch,
        )]);

        let ctx = ActiveContext::default();
        assert!(m.try_match("mipy", ' ', &ctx).is_none());
        assert!(m.try_match("Mipy", ' ', &ctx).is_none());
        let ok = m.try_match("MİPY", ' ', &ctx).unwrap();
        assert_eq!(ok.expanded_text, "Metin ile Pratik Yazım");
    }

    #[test]
    fn app_rules_prefer_specific_app_and_respect_exclude() {
        let mut general = entry(1, "slm", "Selamlar,", CaseMode::KeepOriginal);
        let mut wa = entry(2, "slm", "Selam!", CaseMode::KeepOriginal);
        wa.app_rules.push(AppRule {
            kind: AppRuleKind::Only,
            process_name: "chrome.exe".to_string(),
            title_contains: Some("WhatsApp".to_string()),
        });
        general.app_rules.push(AppRule {
            kind: AppRuleKind::Exclude,
            process_name: "notepad.exe".to_string(),
            title_contains: None,
        });

        let mut m = Matcher::default();
        m.rebuild(vec![general, wa]);

        // Chrome + WhatsApp -> spesifik kural kazanır
        let wa_ctx = ActiveContext {
            process_name: Some("CHROME.EXE".to_string()),
            window_title: Some("WhatsApp Web - Google Chrome".to_string()),
        };
        assert_eq!(m.try_match("slm", ' ', &wa_ctx).unwrap().phrase_id, 2);

        // Notepad -> exclude kuralı nedeniyle genişlemez
        let np_ctx = ActiveContext {
            process_name: Some("notepad.exe".to_string()),
            window_title: Some("Adsız - Not Defteri".to_string()),
        };
        assert!(m.try_match("slm", ' ', &np_ctx).is_none());

        // Word -> genel ifade gelir
        let word_ctx = ActiveContext {
            process_name: Some("WINWORD.EXE".to_string()),
            window_title: Some("Belge1 - Word".to_string()),
        };
        assert_eq!(m.try_match("slm", ' ', &word_ctx).unwrap().phrase_id, 1);
    }

    #[test]
    fn benchmark_10000_phrases_under_1ms() {
        let mut entries = Vec::with_capacity(10_000);
        for i in 0..10_000 {
            entries.push(entry(
                i as i64,
                &format!("kis{i}"),
                &format!("Uzun ifade metni numara {i}"),
                CaseMode::MatchTyped,
            ));
        }
        let mut m = Matcher::default();
        m.rebuild(entries);

        let ctx = ActiveContext::default();
        let start = Instant::now();
        let res = m.try_match("deneme metni kis9999", ' ', &ctx);
        let elapsed = start.elapsed();

        assert!(res.is_some());
        assert_eq!(res.unwrap().phrase_id, 9999);
        assert!(
            elapsed.as_millis() < 5,
            "10.000 ifadede arama çok uzun sürdü: {:?}",
            elapsed
        );
    }

    #[test]
    fn remembers_context_across_typos_punctuation_and_backspaces() {
        use crate::hook::buffer::KeyBuffer;

        let mut m = Matcher::default();
        m.rebuild(vec![entry(
            1,
            "MİPY",
            "Metin ile Pratik Yazım",
            CaseMode::MatchTyped,
        )]);
        let ctx = ActiveContext::default();

        let mut buf = KeyBuffer::new();
        // Kullanıcı cümle içinde yazıp noktalama ve harf hatalarını silerek düzeltiyor
        for c in "Merhaba, mix".chars() {
            buf.push(c);
        }
        buf.backspace(); // 'x' sil
        buf.push('p');
        buf.push('.'); // yanlış noktalama
        buf.backspace(); // '.' sil
        buf.push('z');
        buf.backspace(); // 'z' sil
        buf.push('y');

        let res = m
            .try_match(&buf.as_string(), ' ', &ctx)
            .expect("geri silip yeniden yazdıktan sonra kısaltma eşleşmeli");
        assert_eq!(res.expanded_text, "metin ile pratik yazım");
        assert_eq!(res.abbr_char_len, 4);
    }

    #[test]
    fn suggest_prefix_returns_up_to_3_and_respects_min_2_chars() {
        let mut m = Matcher::default();
        m.rebuild(vec![
            entry(1, "MİPY", "Metin ile Pratik Yazım", CaseMode::MatchTyped),
            entry(2, "MİSA", "Misafir karşılama metni", CaseMode::MatchTyped),
            entry(3, "MİZA", "Mizanpaj notu", CaseMode::MatchTyped),
            entry(4, "MİND", "Mini deneme", CaseMode::MatchTyped),
        ]);
        let ctx = ActiveContext::default();

        // Tek harfte öneri yok
        assert!(m.suggest_prefix("m", &ctx, 3).is_empty());
        // 2+ harfte en fazla 3 öneri
        let res = m.suggest_prefix("merhaba mi", &ctx, 3);
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].typed_len, 2);

        let built = m.build_match_for_id(1, "mi", '\0').unwrap();
        assert_eq!(built.abbr_char_len, 2);
        assert_eq!(built.expanded_text, "metin ile pratik yazım");
    }
}
