//! Türkçe (tr-TR) büyük/küçük harf dönüşümleri ve harf uyumu kuralları (§3.2).
//!
//! Rust'ın standart `to_uppercase()` / `to_lowercase()` fonksiyonları Türkçe'de
//! `i ↔ İ` ve `ı ↔ I` ayrımını bozduğu için çekirdekte hiç kullanılmaz.

use serde::{Deserialize, Serialize};

/// Her ifade için veya varsayılan ayarda seçilebilen harf uyumu modu (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    /// Yazışa uy (varsayılan): `cipy` → küçük, `Cipy` → ilk harf büyük, `CİPY` → tümü büyük.
    #[default]
    MatchTyped,
    /// Aynen getir: kısaltma büyük/küçük fark etmeksizin eşleşir, ifade kayıtlı hâliyle gelir.
    KeepOriginal,
    /// Birebir eşleşme: kısaltma tam olarak kayıtlı harf biçimiyle yazılmalıdır.
    ExactMatch,
}

impl CaseMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CaseMode::MatchTyped => "match_typed",
            CaseMode::KeepOriginal => "keep_original",
            CaseMode::ExactMatch => "exact_match",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "keep_original" => CaseMode::KeepOriginal,
            "exact_match" => CaseMode::ExactMatch,
            _ => CaseMode::MatchTyped,
        }
    }
}

/// Tek karakteri Türkçe kurallarına göre büyütür (`i → İ`, `ı → I`).
#[inline]
pub fn tr_upper_char(c: char) -> char {
    match c {
        'i' => 'İ',
        'ı' => 'I',
        'ç' => 'Ç',
        'ğ' => 'Ğ',
        'ö' => 'Ö',
        'ş' => 'Ş',
        'ü' => 'Ü',
        'â' => 'Â',
        'î' => 'Î',
        'û' => 'Û',
        'a'..='z' => ((c as u8) - b'a' + b'A') as char,
        _ => c,
    }
}

/// Tek karakteri Türkçe kurallarına göre küçültür (`İ → i`, `I → ı`).
#[inline]
pub fn tr_lower_char(c: char) -> char {
    match c {
        'İ' => 'i',
        'I' => 'ı',
        'Ç' => 'ç',
        'Ğ' => 'ğ',
        'Ö' => 'ö',
        'Ş' => 'ş',
        'Ü' => 'ü',
        'Â' => 'â',
        'Î' => 'î',
        'Û' => 'û',
        'A'..='Z' => ((c as u8) - b'A' + b'a') as char,
        _ => c,
    }
}

/// Türkçe büyük harf dönüşümü.
pub fn tr_upper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        out.push(tr_upper_char(c));
    }
    out
}

/// Türkçe küçük harf dönüşümü.
/// Unicode'da birleşik `I\u{0307}` (büyük I + nokta) gelirse tek `i` olarak ele alır.
pub fn tr_lower(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == 'I' && chars.peek() == Some(&'\u{0307}') {
            chars.next();
            out.push('i');
        } else {
            out.push(tr_lower_char(c));
        }
    }
    out
}

/// İlk harfi Türkçe büyük, kalanını Türkçe küçük yapar (cümle düzeni).
/// Örn: "Claude ile Proje Yapmak" → "Claude ile proje yapmak".
pub fn tr_title(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut first_done = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if !first_done && c.is_alphabetic() {
            out.push(tr_upper_char(c));
            first_done = true;
            // Birleşik nokta varsa tüket
            if c == 'I' && chars.peek() == Some(&'\u{0307}') {
                chars.next();
            }
        } else if c == 'I' && chars.peek() == Some(&'\u{0307}') {
            chars.next();
            out.push('i');
        } else {
            out.push(tr_lower_char(c));
        }
    }
    out
}

/// Kısaltmayı indeksleme / arama için normalize eder.
/// `ignore_accents == true` ise Türkçe aksanlı harfleri (`ı→i`, `ç→c`, `ğ→g`, `ö→o`, `ş→s`, `ü→u`, `â→a`, `î→i`, `û→u`) eşitler.
pub fn normalize_abbr(s: &str, ignore_accents: bool) -> String {
    let lowered = tr_lower(s);
    if !ignore_accents {
        return lowered;
    }
    let mut out = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        match ch {
            'ı' | 'î' => out.push('i'),
            'ç' => out.push('c'),
            'ğ' => out.push('g'),
            'ö' => out.push('o'),
            'ş' => out.push('s'),
            'ü' | 'û' => out.push('u'),
            'â' => out.push('a'),
            other => out.push(other),
        }
    }
    out
}

/// Karakterin Türkçe büyük harf olup olmadığını döndürür.
#[inline]
fn is_tr_upper(c: char) -> bool {
    matches!(
        c,
        'A'..='Z' | 'İ' | 'Ç' | 'Ğ' | 'Ö' | 'Ş' | 'Ü' | 'Â' | 'Î' | 'Û'
    )
}

/// Karakterin Türkçe küçük harf olup olmadığını döndürür.
#[inline]
fn is_tr_lower(c: char) -> bool {
    matches!(
        c,
        'a'..='z' | 'ı' | 'ç' | 'ğ' | 'ö' | 'ş' | 'ü' | 'â' | 'î' | 'û'
    )
}

/// Metni Word'deki `Shift+F3` mantığıyla Türkçe harf kurallarına göre döndürür:
/// - Tümü küçük → İlk harf büyük (`tr_title`)
/// - İlk harf büyük, kalanı küçük → TÜMÜ BÜYÜK (`tr_upper`)
/// - Diğer (tümü büyük veya karışık) → tümü küçük (`tr_lower`)
pub fn cycle_turkish_case(text: &str) -> String {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return text.to_string();
    }

    let all_lower = letters.iter().all(|&c| is_tr_lower(c));
    if all_lower {
        return tr_title(text);
    }

    if is_tr_upper(letters[0]) && letters[1..].iter().all(|&c| is_tr_lower(c)) {
        return tr_upper(text);
    }

    tr_lower(text)
}

/// Yazılan kısaltmanın harf kalıbına göre genişletilmiş metni biçimlendirir (§3.2).
pub fn apply_case(typed: &str, expansion: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::KeepOriginal | CaseMode::ExactMatch => expansion.to_string(),
        CaseMode::MatchTyped => {
            let letters: Vec<char> = typed.chars().filter(|c| c.is_alphabetic()).collect();
            if letters.is_empty() {
                return expansion.to_string();
            }

            let all_lower = letters.iter().all(|&c| is_tr_lower(c));
            if all_lower {
                return tr_lower(expansion);
            }

            let all_upper = letters.iter().all(|&c| is_tr_upper(c));
            if all_upper {
                // Tek harfli büyük kısaltma (ör. "İ") cümle başı gibi mi tümü büyük mü?
                // Tek harfse tümü büyük yerine ilk harf büyük daha doğal; 2+ harfse tümü büyük.
                if letters.len() == 1 {
                    return tr_title(expansion);
                }
                return tr_upper(expansion);
            }

            // İlk harf büyük, kalanı küçükse (ör. "Cipy") → cümle düzeni
            if is_tr_upper(letters[0]) && letters[1..].iter().all(|&c| is_tr_lower(c)) {
                return tr_title(expansion);
            }

            // Karışık harf (ör. "cİpy" veya "CiPy") → kayıtlı metni koru
            expansion.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_upper_and_lower_i_dot_rules() {
        assert_eq!(tr_upper("cipy"), "CİPY");
        assert_eq!(tr_upper("cıpy"), "CIPY");
        assert_eq!(tr_lower("CİPY"), "cipy");
        assert_eq!(tr_lower("CIPY"), "cıpy");
        assert_eq!(tr_upper("istanbul ığdır çğöşü"), "İSTANBUL IĞDIR ÇĞÖŞÜ");
        assert_eq!(tr_lower("İSTANBUL IĞDIR ÇĞÖŞÜ"), "istanbul ığdır çğöşü");
    }

    #[test]
    fn tr_lower_handles_combining_dot_above() {
        // Bazı sistemlerde İ, 'I' + '\u{0307}' olarak gelebilir
        assert_eq!(tr_lower("CI\u{0307}PY"), "cipy");
    }

    #[test]
    fn spec_section_3_2_table_all_cells() {
        let expansion = "Claude ile Proje Yapmak";

        // Yazışa uy (MatchTyped)
        assert_eq!(
            apply_case("cipy", expansion, CaseMode::MatchTyped),
            "claude ile proje yapmak"
        );
        assert_eq!(
            apply_case("Cipy", expansion, CaseMode::MatchTyped),
            "Claude ile proje yapmak"
        );
        assert_eq!(
            apply_case("CİPY", expansion, CaseMode::MatchTyped),
            "CLAUDE İLE PROJE YAPMAK"
        );

        // Aynen getir (KeepOriginal)
        assert_eq!(
            apply_case("cipy", expansion, CaseMode::KeepOriginal),
            "Claude ile Proje Yapmak"
        );
        assert_eq!(
            apply_case("Cipy", expansion, CaseMode::KeepOriginal),
            "Claude ile Proje Yapmak"
        );
        assert_eq!(
            apply_case("CİPY", expansion, CaseMode::KeepOriginal),
            "Claude ile Proje Yapmak"
        );

        // Birebir eşleşme (ExactMatch) — eşleştiğinde metin aynen gelir
        assert_eq!(
            apply_case("CİPY", expansion, CaseMode::ExactMatch),
            "Claude ile Proje Yapmak"
        );
    }

    #[test]
    fn normalize_separates_dotted_and_dotless_by_default() {
        assert_eq!(normalize_abbr("CİPY", false), "cipy");
        assert_eq!(normalize_abbr("cipy", false), "cipy");
        assert_eq!(normalize_abbr("CIPY", false), "cıpy");
        assert_eq!(normalize_abbr("cıpy", false), "cıpy");
        assert_ne!(normalize_abbr("CİPY", false), normalize_abbr("CIPY", false));

        // ignore_accents açıkken Türkçe aksanlı harfler eşitlenir
        assert_eq!(normalize_abbr("CİPY", true), "cipy");
        assert_eq!(normalize_abbr("CIPY", true), "cipy");
        assert_eq!(normalize_abbr("ÇİÇEK", true), "cicek");
        assert_eq!(normalize_abbr("ŞİRKET", true), "sirket");
        assert_eq!(normalize_abbr("IŞIK", true), "isik");
        assert_eq!(normalize_abbr("ÖĞÜN", true), "ogun");
    }

    #[test]
    fn mixed_case_preserves_original() {
        let expansion = "Claude ile Proje Yapmak";
        assert_eq!(apply_case("cİpy", expansion, CaseMode::MatchTyped), expansion);
        assert_eq!(apply_case("CiPy", expansion, CaseMode::MatchTyped), expansion);
    }

    #[test]
    fn tr_title_turkish_initials() {
        assert_eq!(tr_title("istanbul Büyük"), "İstanbul büyük");
        assert_eq!(tr_title("ığdır İli"), "Iğdır ili");
    }

    #[test]
    fn cycle_turkish_case_works() {
        let s1 = cycle_turkish_case("istanbul ığdır");
        assert_eq!(s1, "İstanbul ığdır");
        let s2 = cycle_turkish_case(&s1);
        assert_eq!(s2, "İSTANBUL IĞDIR");
        let s3 = cycle_turkish_case(&s2);
        assert_eq!(s3, "istanbul ığdır");
        assert_eq!(cycle_turkish_case("Karışık Metin"), "karışık metin");
    }
}
