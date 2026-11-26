//! Değişken ve doldurulacak alan çözümleyici (§3.3, M4.1).
//!
//! Desteklenen sözdizimi:
//! - `{tarih}` → `26.11.2026`
//! - `{tarih:uzun}` → `26 Kasım 2026 Perşembe`
//! - `{tarih:+1g}` / `{tarih:-1a}` / `{tarih:+2h}` / `{tarih:-1y}` (`g`=gün, `h`=hafta, `a`=ay, `y`=yıl)
//! - `{saat}` → `18:05`
//! - `{pano}` → panodaki metin
//! - `{imleç}` → genişletmeden sonra imlecin duracağı yer
//! - `{alan:İsim}` / `{alan:Durum|Acil,Normal,Düşük}` → form alanı
//! - `{ifade:KISALTMA}` → başka ifadeyi gömer (döngü korumalı)

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, Timelike, Weekday};
use serde::{Deserialize, Serialize};

use crate::matcher::case::normalize_abbr;

const MAX_PHRASE_DEPTH: usize = 8;

/// Doldurulacak alan tanımı (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub options: Option<Vec<String>>,
}

pub type PhraseLookupFn<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// Şablon çözümleme bağlamı.
pub struct ResolveContext<'a> {
    pub now: DateTime<Local>,
    pub clipboard_text: Option<String>,
    pub field_values: HashMap<String, String>,
    pub fold_dotted_i: bool,
    pub phrase_lookup: PhraseLookupFn<'a>,
}

/// Çözümlenmiş şablon çıktısı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutput {
    pub text: String,
    /// Metnin sonundan sola doğru kaç `←` tuşu gönderileceği (`{imleç}` için).
    pub cursor_left_steps: usize,
}

fn turkish_month_name(month: u32) -> &'static str {
    match month {
        1 => "Ocak",
        2 => "Şubat",
        3 => "Mart",
        4 => "Nisan",
        5 => "Mayıs",
        6 => "Haziran",
        7 => "Temmuz",
        8 => "Ağustos",
        9 => "Eylül",
        10 => "Ekim",
        11 => "Kasım",
        12 => "Aralık",
        _ => "",
    }
}

fn turkish_weekday_name(wd: Weekday) -> &'static str {
    match wd {
        Weekday::Mon => "Pazartesi",
        Weekday::Tue => "Salı",
        Weekday::Wed => "Çarşamba",
        Weekday::Thu => "Perşembe",
        Weekday::Fri => "Cuma",
        Weekday::Sat => "Cumartesi",
        Weekday::Sun => "Pazar",
    }
}

fn format_date_short(d: NaiveDate) -> String {
    format!("{:02}.{:02}.{:04}", d.day(), d.month(), d.year())
}

fn format_date_long(d: NaiveDate) -> String {
    format!(
        "{} {} {} {}",
        d.day(),
        turkish_month_name(d.month()),
        d.year(),
        turkish_weekday_name(d.weekday())
    )
}

fn shift_date(base: NaiveDate, spec: &str) -> Option<NaiveDate> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Some(base);
    }
    let sign: i32 = if trimmed.starts_with('-') {
        -1
    } else if trimmed.starts_with('+') {
        1
    } else {
        return None;
    };

    let rest = &trimmed[1..];
    if rest.is_empty() {
        return None;
    }
    let mut chars = rest.chars();
    let unit = chars.next_back()?;
    let num_str: String = chars.collect();
    let amount: u32 = num_str.parse().ok()?;

    match unit {
        'g' | 'G' => {
            let days = Days::new(amount as u64);
            if sign > 0 {
                base.checked_add_days(days)
            } else {
                base.checked_sub_days(days)
            }
        }
        'h' | 'H' => {
            let days = Days::new((amount as u64).saturating_mul(7));
            if sign > 0 {
                base.checked_add_days(days)
            } else {
                base.checked_sub_days(days)
            }
        }
        'a' | 'A' => {
            let months = Months::new(amount);
            if sign > 0 {
                base.checked_add_months(months)
            } else {
                base.checked_sub_months(months)
            }
        }
        'y' | 'Y' => {
            let months = Months::new(amount.saturating_mul(12));
            if sign > 0 {
                base.checked_add_months(months)
            } else {
                base.checked_sub_months(months)
            }
        }
        _ => None,
    }
}

fn parse_field_token(inner: &str) -> Option<FormField> {
    let spec = inner.strip_prefix("alan:")?;
    if let Some((raw_name, raw_opts)) = spec.split_once('|') {
        let name = raw_name.trim().to_string();
        if name.is_empty() {
            return None;
        }
        let options: Vec<String> = raw_opts
            .split(',')
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .collect();
        Some(FormField {
            name,
            options: if options.is_empty() {
                None
            } else {
                Some(options)
            },
        })
    } else {
        let name = spec.trim().to_string();
        if name.is_empty() {
            None
        } else {
            Some(FormField {
                name,
                options: None,
            })
        }
    }
}

/// Şablondaki `{alan:...}` doldurulacak alanlarını sırayla ve tekilleştirerek çıkarır.
pub fn extract_form_fields(template: &str) -> Vec<FormField> {
    let mut fields = Vec::new();
    let mut seen = HashSet::new();
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            break;
        };
        let token = &after_open[..close];
        if let Some(field) = parse_field_token(token) {
            if seen.insert(field.name.clone()) {
                fields.push(field);
            }
        }
        rest = &after_open[close + 1..];
    }
    fields
}

fn resolve_recursive(
    template: &str,
    ctx: &ResolveContext<'_>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };

        let token = &after_open[..close];
        if token == "tarih" {
            out.push_str(&format_date_short(ctx.now.date_naive()));
        } else if token == "tarih:uzun" {
            out.push_str(&format_date_long(ctx.now.date_naive()));
        } else if let Some(offset_spec) = token.strip_prefix("tarih:") {
            if let Some(shifted) = shift_date(ctx.now.date_naive(), offset_spec) {
                out.push_str(&format_date_short(shifted));
            } else {
                out.push('{');
                out.push_str(token);
                out.push('}');
            }
        } else if token == "saat" {
            let t = ctx.now.time();
            out.push_str(&format!("{:02}:{:02}", t.hour(), t.minute()));
        } else if token == "pano" {
            if let Some(clip) = &ctx.clipboard_text {
                out.push_str(clip);
            }
        } else if token == "imleç" {
            // Son adımda konum hesaplamak için benzersiz işaretçi bırak
            out.push('\u{E000}');
        } else if let Some(field) = parse_field_token(token) {
            if let Some(val) = ctx.field_values.get(&field.name) {
                out.push_str(val);
            }
        } else if let Some(ref_abbr) = token.strip_prefix("ifade:") {
            let norm = normalize_abbr(ref_abbr.trim(), ctx.fold_dotted_i);
            if depth < MAX_PHRASE_DEPTH && !norm.is_empty() && !visited.contains(&norm) {
                if let Some(embedded) = (ctx.phrase_lookup)(ref_abbr.trim()) {
                    visited.insert(norm.clone());
                    let resolved = resolve_recursive(&embedded, ctx, visited, depth + 1);
                    out.push_str(&resolved);
                    visited.remove(&norm);
                }
            }
        } else {
            out.push('{');
            out.push_str(token);
            out.push('}');
        }

        rest = &after_open[close + 1..];
    }

    out.push_str(rest);
    out
}

/// Şablondaki tüm değişkenleri, iç içe ifadeleri ve `{imleç}` konumunu çözümler.
pub fn resolve_template(template: &str, ctx: &ResolveContext<'_>) -> ResolvedOutput {
    let mut visited = HashSet::new();
    let raw = resolve_recursive(template, ctx, &mut visited, 0);

    if let Some(marker_pos) = raw.find('\u{E000}') {
        let before = &raw[..marker_pos];
        let after = raw[marker_pos + '\u{E000}'.len_utf8()..].replace('\u{E000}', "");
        let cursor_left_steps = after.chars().count();
        let mut text = String::with_capacity(before.len() + after.len());
        text.push_str(before);
        text.push_str(&after);
        ResolvedOutput {
            text,
            cursor_left_steps,
        }
    } else {
        ResolvedOutput {
            text: raw,
            cursor_left_steps: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_ctx<'a>(
        y: i32,
        m: u32,
        d: u32,
        hh: u32,
        mm: u32,
        lookup: impl Fn(&str) -> Option<String> + 'a,
    ) -> ResolveContext<'a> {
        let now = Local.with_ymd_and_hms(y, m, d, hh, mm, 0).unwrap();
        ResolveContext {
            now,
            clipboard_text: Some("Panodan Metin".to_string()),
            field_values: HashMap::from([
                ("İsim".to_string(), "Örnek Kişi".to_string()),
                ("Durum".to_string(), "Acil".to_string()),
            ]),
            fold_dotted_i: false,
            phrase_lookup: Box::new(lookup),
        }
    }

    #[test]
    fn resolves_dates_times_clipboard_fields_and_cursor() {
        let ctx = make_ctx(2026, 11, 26, 18, 5, |_| None);
        let tpl = "Sayın {alan:İsim} ({alan:Durum|Acil,Normal}), {tarih} / {tarih:uzun} {saat} - {pano} [{imleç}]";
        let out = resolve_template(tpl, &ctx);

        assert_eq!(
            out.text,
            "Sayın Örnek Kişi (Acil), 26.11.2026 / 26 Kasım 2026 Perşembe 18:05 - Panodan Metin []"
        );
        // Sonda yalnızca "]" kaldı → 1 kez sol ok
        assert_eq!(out.cursor_left_steps, 1);
    }

    #[test]
    fn handles_month_end_and_leap_year_date_shifts() {
        // 31 Ocak 2024 + 1 ay → 29 Şubat 2024 (artık yıl!)
        let leap = make_ctx(2024, 1, 31, 9, 0, |_| None);
        assert_eq!(resolve_template("{tarih:+1a}", &leap).text, "29.02.2024");

        // 31 Ocak 2026 + 1 ay → 28 Şubat 2026 (artık yıl değil)
        let non_leap = make_ctx(2026, 1, 31, 9, 0, |_| None);
        assert_eq!(
            resolve_template("{tarih:+1a}", &non_leap).text,
            "28.02.2026"
        );
        assert_eq!(
            resolve_template("{tarih:+1g} {tarih:-1h} {tarih:+1y}", &non_leap).text,
            "01.02.2026 24.01.2026 31.01.2027"
        );
    }

    #[test]
    fn resolves_nested_phrases_and_stops_cycles() {
        let ctx = make_ctx(2026, 11, 26, 10, 0, |abbr| match abbr {
            "A" => Some("Baş->{ifade:B}".to_string()),
            "B" => Some("Orta->{ifade:A}".to_string()),
            "İMZA" => Some("Örnek İmza".to_string()),
            _ => None,
        });

        // Döngüde sonsuz özyinelemeye girmeden güvenli durmalı
        let cyc = resolve_template("{ifade:A}", &ctx);
        assert_eq!(cyc.text, "Baş->Orta->");

        let ok = resolve_template("Saygılar, {ifade:İMZA}", &ctx);
        assert_eq!(ok.text, "Saygılar, Örnek İmza");
    }

    #[test]
    fn extracts_unique_form_fields_preserving_order() {
        let fields = extract_form_fields(
            "Merhaba {alan:İsim}, {alan:Durum|Acil, Normal, Düşük} görevi için {alan:İsim} hazır.",
        );
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "İsim");
        assert_eq!(fields[0].options, None);
        assert_eq!(fields[1].name, "Durum");
        assert_eq!(
            fields[1].options,
            Some(vec![
                "Acil".to_string(),
                "Normal".to_string(),
                "Düşük".to_string()
            ])
        );
    }
}
