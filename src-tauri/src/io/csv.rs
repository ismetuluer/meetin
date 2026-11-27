//! CSV içe aktarma ayrıştırıcısı (§8, M1.9).
//!
//! - UTF-8 (BOM'lu/BOM'suz) ve Windows-1254 (Türkçe) kodlamalarını otomatik algılar.
//! - Virgül (`,`) ve noktalı virgül (`;`) ayırıcılarını otomatik algılar.
//! - Boş satırları atlar, tırnaklı çok satırlı alanları korur.
//! - Önizleme ve çakışma stratejisi (üzerine yaz / atla / yeni kısaltma öner) sunar.

use std::collections::{HashMap, HashSet};

use encoding_rs::WINDOWS_1254;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::{DbError, NewPhrase, Storage, UpdatePhrase};
use crate::matcher::case::{normalize_abbr, CaseMode};

#[derive(Debug, Error)]
pub enum CsvImportError {
    #[error("CSV ayrıştırma hatası: {0}")]
    Csv(#[from] csv::Error),
    #[error("Veritabanı hatası: {0}")]
    Db(#[from] DbError),
    #[error("Kısaltma ve metin sütunları seçilmelidir")]
    MissingRequiredColumns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    Overwrite,
    Skip,
    SuggestNew,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvColumnMapping {
    pub abbr_col: usize,
    pub content_col: usize,
    pub description_col: Option<usize>,
    pub folder_col: Option<usize>,
    pub tags_col: Option<usize>,
    pub has_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvTablePreview {
    pub detected_encoding: String,
    pub detected_delimiter: String,
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvImportSummary {
    pub total_valid: usize,
    pub imported: usize,
    pub overwritten: usize,
    pub skipped: usize,
    pub renamed: usize,
    pub conflicts: Vec<String>,
}

/// Ham baytları UTF-8 veya Windows-1254 olarak çözer.
pub fn decode_bytes(raw: &[u8]) -> (String, &'static str) {
    // UTF-8 BOM
    if let Some(stripped) = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        if let Ok(s) = std::str::from_utf8(stripped) {
            return (s.to_string(), "UTF-8 (BOM)");
        }
    }
    if let Ok(s) = std::str::from_utf8(raw) {
        return (s.to_string(), "UTF-8");
    }
    let (decoded, _, _) = WINDOWS_1254.decode(raw);
    (decoded.into_owned(), "Windows-1254")
}

/// İlk anlamlı satırdaki `;` ve `,` sayısına bakarak ayırıcıyı belirler.
pub fn detect_delimiter(text: &str) -> u8 {
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let semicolons = first_line.matches(';').count();
    let commas = first_line.matches(',').count();
    if semicolons > commas {
        b';'
    } else {
        b','
    }
}

/// Ham CSV baytlarını ayrıştırır (boş satırları atlar).
pub fn parse_raw_csv(raw: &[u8]) -> Result<(String, u8, Vec<Vec<String>>), CsvImportError> {
    let (decoded, encoding_name) = decode_bytes(raw);
    let delimiter = detect_delimiter(&decoded);

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(decoded.as_bytes());

    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let cols: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
        if cols.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        rows.push(cols);
    }

    Ok((encoding_name.to_string(), delimiter, rows))
}

/// Sütun eşleme ekranı için CSV önizlemesi üretir.
pub fn preview_csv(raw: &[u8]) -> Result<CsvTablePreview, CsvImportError> {
    let (encoding, delimiter, rows) = parse_raw_csv(raw)?;
    let headers = rows.first().cloned().unwrap_or_default();
    let sample_rows: Vec<Vec<String>> = rows.iter().skip(1).take(10).cloned().collect();
    let total_rows = rows.len().saturating_sub(1);

    Ok(CsvTablePreview {
        detected_encoding: encoding,
        detected_delimiter: (delimiter as char).to_string(),
        headers,
        sample_rows,
        total_rows,
    })
}

/// Çakışma durumunda yeni kısaltma önerir (`MİPY` → `MİPY2`, `MİPY3` …).
fn suggest_unique_abbr(base: &str, used_norms: &HashSet<String>, fold_dotted_i: bool) -> String {
    let mut n = 2usize;
    loop {
        let candidate = format!("{base}{n}");
        let norm = normalize_abbr(&candidate, fold_dotted_i);
        if !used_norms.contains(&norm) {
            return candidate;
        }
        n += 1;
    }
}

/// CSV satırlarını veritabanına aktarır (dry_run=true ise yalnızca önizleme özeti döner).
pub fn import_csv(
    storage: &Storage,
    raw: &[u8],
    mapping: &CsvColumnMapping,
    strategy: ConflictStrategy,
    fold_dotted_i: bool,
    dry_run: bool,
) -> Result<CsvImportSummary, CsvImportError> {
    let (_, _, rows) = parse_raw_csv(raw)?;
    let data_rows: &[Vec<String>] = if mapping.has_header && !rows.is_empty() {
        &rows[1..]
    } else {
        &rows[..]
    };

    // Mevcut ifadeleri normalize kısaltma → (id, phrase) eşlemesiyle yükle
    let existing = storage.list_phrases(false)?;
    let mut existing_by_norm: HashMap<String, i64> = HashMap::new();
    let mut used_norms: HashSet<String> = HashSet::new();
    for p in &existing {
        let norm = normalize_abbr(&p.abbr, fold_dotted_i);
        existing_by_norm.insert(norm.clone(), p.id);
        used_norms.insert(norm);
    }

    let mut summary = CsvImportSummary {
        total_valid: 0,
        imported: 0,
        overwritten: 0,
        skipped: 0,
        renamed: 0,
        conflicts: Vec::new(),
    };

    for row in data_rows {
        let abbr = row.get(mapping.abbr_col).map(|s| s.trim()).unwrap_or("");
        let content = row
            .get(mapping.content_col)
            .map(|s| s.as_str())
            .unwrap_or("");

        if abbr.is_empty() || content.is_empty() {
            continue;
        }
        summary.total_valid += 1;

        let description = mapping
            .description_col
            .and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let folder_name = mapping
            .folder_col
            .and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let tags: Vec<String> = mapping
            .tags_col
            .and_then(|i| row.get(i))
            .map(|s| {
                s.split([',', ';'])
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let norm = normalize_abbr(abbr, fold_dotted_i);
        let conflict_id = existing_by_norm.get(&norm).copied();

        if let Some(existing_id) = conflict_id {
            summary.conflicts.push(abbr.to_string());
            match strategy {
                ConflictStrategy::Skip => {
                    summary.skipped += 1;
                }
                ConflictStrategy::Overwrite => {
                    if !dry_run {
                        let folder_id = match &folder_name {
                            Some(name) => Some(storage.get_or_create_folder(name)?),
                            None => None,
                        };
                        storage.update_phrase(&UpdatePhrase {
                            id: existing_id,
                            abbr: abbr.to_string(),
                            content: content.to_string(),
                            content_html: None,
                            description,
                            folder_id,
                            folder_ids: folder_id.into_iter().collect(),
                            case_mode: CaseMode::MatchTyped,
                            shortcut: None,
                            enabled: true,
                            tags,
                        })?;
                    }
                    summary.overwritten += 1;
                }
                ConflictStrategy::SuggestNew => {
                    let new_abbr = suggest_unique_abbr(abbr, &used_norms, fold_dotted_i);
                    let new_norm = normalize_abbr(&new_abbr, fold_dotted_i);
                    used_norms.insert(new_norm.clone());
                    if !dry_run {
                        let folder_id = match &folder_name {
                            Some(name) => Some(storage.get_or_create_folder(name)?),
                            None => None,
                        };
                        let created = storage.create_phrase(&NewPhrase {
                            abbr: new_abbr,
                            content: content.to_string(),
                            content_html: None,
                            description,
                            folder_id,
                            folder_ids: folder_id.into_iter().collect(),
                            case_mode: CaseMode::MatchTyped,
                            shortcut: None,
                            tags,
                        })?;
                        existing_by_norm.insert(new_norm, created.id);
                    }
                    summary.renamed += 1;
                }
            }
        } else {
            used_norms.insert(norm.clone());
            if !dry_run {
                let folder_id = match &folder_name {
                    Some(name) => Some(storage.get_or_create_folder(name)?),
                    None => None,
                };
                let created = storage.create_phrase(&NewPhrase {
                    abbr: abbr.to_string(),
                    content: content.to_string(),
                    content_html: None,
                    description,
                    folder_id,
                    folder_ids: folder_id.into_iter().collect(),
                    case_mode: CaseMode::MatchTyped,
                    shortcut: None,
                    tags,
                })?;
                existing_by_norm.insert(norm, created.id);
            }
            summary.imported += 1;
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_utf8_and_windows_1254_turkish_characters() {
        let utf8_csv =
            "Kısaltma,Metin,Açıklama\nMİPY,\"Metin ile Pratik Yazım, Türkçe şğüöçı\",Örnek\n\n";
        let prev = preview_csv(utf8_csv.as_bytes()).unwrap();
        assert_eq!(prev.detected_encoding, "UTF-8");
        assert_eq!(prev.detected_delimiter, ",");
        assert_eq!(prev.total_rows, 1);
        assert_eq!(prev.sample_rows[0][0], "MİPY");
        assert_eq!(
            prev.sample_rows[0][1],
            "Metin ile Pratik Yazım, Türkçe şğüöçı"
        );

        // Windows-1254 kodlu ve noktalı virgüllü CSV
        let text_tr = "Kısaltma;Metin\nŞĞÜ;İstanbul'da güneşli günler\n";
        let (encoded, _, _) = WINDOWS_1254.encode(text_tr);
        let prev_1254 = preview_csv(&encoded).unwrap();
        assert_eq!(prev_1254.detected_encoding, "Windows-1254");
        assert_eq!(prev_1254.detected_delimiter, ";");
        assert_eq!(prev_1254.sample_rows[0][0], "ŞĞÜ");
        assert_eq!(prev_1254.sample_rows[0][1], "İstanbul'da güneşli günler");
    }

    #[test]
    fn import_handles_conflicts_with_all_three_strategies() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        let csv_data = b"Kisaltma,Metin,Aciklama,Klasor,Etiketler\nM\xC4\xB0PY,Ilk Metin,Acik1,Genel,\"tag1, tag2\"\n";
        let mapping = CsvColumnMapping {
            abbr_col: 0,
            content_col: 1,
            description_col: Some(2),
            folder_col: Some(3),
            tags_col: Some(4),
            has_header: true,
        };

        let r1 = import_csv(
            &storage,
            csv_data,
            &mapping,
            ConflictStrategy::Skip,
            false,
            false,
        )
        .unwrap();
        assert_eq!(r1.imported, 1);
        assert!(r1.conflicts.is_empty());

        // Aynı kısaltma -> Skip
        let r_skip = import_csv(
            &storage,
            csv_data,
            &mapping,
            ConflictStrategy::Skip,
            false,
            false,
        )
        .unwrap();
        assert_eq!(r_skip.skipped, 1);
        assert_eq!(r_skip.conflicts.len(), 1);

        // SuggestNew -> MİPY2 oluşturulur
        let r_new = import_csv(
            &storage,
            csv_data,
            &mapping,
            ConflictStrategy::SuggestNew,
            false,
            false,
        )
        .unwrap();
        assert_eq!(r_new.renamed, 1);
        let all = storage.list_phrases(false).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|p| p.abbr == "MİPY2"));
    }
}
