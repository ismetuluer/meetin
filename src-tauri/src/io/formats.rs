//! Çok biçimli içe ve dışa aktarma modülü (§8, M5.6).
//!
//! Desteklenen biçimler:
//! - `.ahk` (AutoHotkey hotstringleri)
//! - `.txt` (boş satırla ayrılmış metin blokları)
//! - `.json` / `.meetin` (Meetin yerel yedeği + Beeftext JSON)
//! - `.csv` dışa aktarım (UTF-8 BOM)

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::{DbError, FolderRecord, NewPhrase, PhraseRecord, Storage, UpdatePhrase};
use crate::io::csv::{decode_bytes, ConflictStrategy, CsvImportSummary};
use crate::io::mee_archive::{import_mee_archive, looks_like_zip, MeeArchiveError};
use crate::matcher::case::{normalize_abbr, tr_upper, CaseMode};

#[derive(Debug, Error)]
pub enum FormatIoError {
    #[error("JSON ayrıştırma hatası: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV yazma hatası: {0}")]
    Csv(#[from] csv::Error),
    #[error("Veritabanı hatası: {0}")]
    Db(#[from] DbError),
    #[error("{0}")]
    Archive(#[from] MeeArchiveError),
    #[error("Desteklenmeyen dosya biçimi: {0}")]
    UnsupportedFormat(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedPhraseItem {
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    pub description: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub case_mode: CaseMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeetinExportDoc {
    format: String,
    version: u32,
    #[serde(default)]
    folders: Vec<String>,
    phrases: Vec<MeetinExportPhrase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeetinExportPhrase {
    abbr: String,
    content: String,
    #[serde(default)]
    content_html: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    case_mode: CaseMode,
}

#[derive(Debug, Clone, Deserialize)]
struct BeeftextDoc {
    #[serde(default)]
    combos: Vec<BeeftextCombo>,
    #[serde(default)]
    groups: Vec<BeeftextGroup>,
}

#[derive(Debug, Clone, Deserialize)]
struct BeeftextGroup {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BeeftextCombo {
    #[serde(default)]
    keyword: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    group: Option<String>,
}

fn unescape_ahk(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '`' {
            match chars.next() {
                Some('n') | Some('r') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('`') => out.push('`'),
                Some(';') => out.push(';'),
                Some(other) => out.push(other),
                None => out.push('`'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn escape_ahk(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.replace("\r\n", "\n").chars() {
        match ch {
            '`' => out.push_str("``"),
            '\n' => out.push_str("`n"),
            '\t' => out.push_str("`t"),
            ';' => out.push_str("`;"),
            other => out.push(other),
        }
    }
    out
}

/// AutoHotkey (`.ahk`) hotstring satırlarını ve çok satırlı bloklarını ayrıştırır.
pub fn parse_ahk_hotstrings(raw: &[u8]) -> Vec<ImportedPhraseItem> {
    let (decoded, _) = decode_bytes(raw);
    let lines: Vec<&str> = decoded.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || !trimmed.starts_with(':') {
            i += 1;
            continue;
        }
        // Biçim: :opts:abbr::replacement
        if let Some(after_first) = trimmed.strip_prefix(':') {
            if let Some(second_colon) = after_first.find(':') {
                let opts = &after_first[..second_colon];
                let rest = &after_first[second_colon + 1..];
                if let Some(sep_idx) = rest.find("::") {
                    let abbr = rest[..sep_idx].trim();
                    let body = rest[sep_idx + 2..].trim_start();
                    if !abbr.is_empty() {
                        let case_mode = if opts.contains('C') {
                            CaseMode::KeepOriginal
                        } else {
                            CaseMode::MatchTyped
                        };
                        let next_is_paren = lines
                            .get(i + 1)
                            .is_some_and(|next| next.trim() == "(");
                        if body == "(" || (body.is_empty() && next_is_paren) {
                            // Çok satırlı AHK devam bloğu
                            if body != "(" {
                                i += 1;
                            }
                            let mut block = Vec::new();
                            i += 1;
                            while i < lines.len() {
                                if lines[i].trim() == ")" {
                                    break;
                                }
                                block.push(lines[i]);
                                i += 1;
                            }
                            let joined = block.join("\n");
                            if !joined.trim().is_empty() {
                                out.push(ImportedPhraseItem {
                                    abbr: abbr.to_string(),
                                    content: joined,
                                    content_html: None,
                                    description: String::new(),
                                    folder: Some("AutoHotkey".to_string()),
                                    tags: vec!["ahk".to_string()],
                                    case_mode,
                                });
                            }
                        } else {
                            let content = unescape_ahk(body);
                            if !content.trim().is_empty() {
                                out.push(ImportedPhraseItem {
                                    abbr: abbr.to_string(),
                                    content,
                                    content_html: None,
                                    description: String::new(),
                                    folder: Some("AutoHotkey".to_string()),
                                    tags: vec!["ahk".to_string()],
                                    case_mode,
                                });
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }

    out
}

fn suggest_abbr_tr(content: &str, index: usize) -> String {
    let mut initials = String::new();
    for word in content.split_whitespace() {
        if let Some(ch) = word.chars().find(|c| c.is_alphanumeric()) {
            initials.push_str(&tr_upper(&ch.to_string()));
            if initials.chars().count() >= 5 {
                break;
            }
        }
    }
    if initials.chars().count() < 2 {
        format!("METİN{}", index + 1)
    } else {
        initials
    }
}

/// Düz metin (`.txt`) içindeki boş satırla ayrılmış blokları kısaltma önerisiyle ayrıştırır.
pub fn parse_txt_blocks(raw: &[u8]) -> Vec<ImportedPhraseItem> {
    let (decoded, _) = decode_bytes(raw);
    let normalized = decoded.replace("\r\n", "\n");
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in normalized.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }

    blocks
        .into_iter()
        .enumerate()
        .filter(|(_, text)| !text.trim().is_empty())
        .map(|(idx, text)| ImportedPhraseItem {
            abbr: suggest_abbr_tr(&text, idx),
            content: text,
            content_html: None,
            description: String::new(),
            folder: Some("Metin".to_string()),
            tags: vec![],
            case_mode: CaseMode::MatchTyped,
        })
        .collect()
}

/// Meetin JSON (`.meetin`/`.json`) veya Beeftext JSON biçimini ayrıştırır.
pub fn parse_beeftext_or_meetin_json(
    raw: &[u8],
) -> Result<Vec<ImportedPhraseItem>, FormatIoError> {
    let (decoded, _) = decode_bytes(raw);
    let trimmed = decoded.trim_start_matches('\u{FEFF}').trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(doc) = serde_json::from_str::<MeetinExportDoc>(trimmed) {
        if !doc.phrases.is_empty() || doc.format == "meetin" {
            return Ok(doc
                .phrases
                .into_iter()
                .filter(|p| !p.abbr.trim().is_empty() && !p.content.trim().is_empty())
                .map(|p| ImportedPhraseItem {
                    abbr: p.abbr.trim().to_string(),
                    content: p.content,
                    content_html: p.content_html,
                    description: p.description,
                    folder: p.folder.filter(|f| !f.trim().is_empty()),
                    tags: p.tags,
                    case_mode: p.case_mode,
                })
                .collect());
        }
    }

    if let Ok(arr) = serde_json::from_str::<Vec<MeetinExportPhrase>>(trimmed) {
        return Ok(arr
            .into_iter()
            .filter(|p| !p.abbr.trim().is_empty() && !p.content.trim().is_empty())
            .map(|p| ImportedPhraseItem {
                abbr: p.abbr.trim().to_string(),
                content: p.content,
                content_html: p.content_html,
                description: p.description,
                folder: p.folder.filter(|f| !f.trim().is_empty()),
                tags: p.tags,
                case_mode: p.case_mode,
            })
            .collect());
    }

    let bt: BeeftextDoc = serde_json::from_str(trimmed)?;
    let group_map: HashMap<String, String> = bt
        .groups
        .into_iter()
        .map(|g| (g.uuid, g.name))
        .collect();

    Ok(bt
        .combos
        .into_iter()
        .filter(|c| !c.keyword.trim().is_empty() && !c.snippet.trim().is_empty())
        .map(|c| {
            let folder = c
                .group
                .and_then(|gid| group_map.get(&gid).cloned().or(Some(gid)))
                .filter(|s| !s.trim().is_empty());
            let description = if c.description.trim().is_empty() {
                c.name
            } else {
                c.description
            };
            ImportedPhraseItem {
                abbr: c.keyword.trim().to_string(),
                content: c.snippet,
                content_html: None,
                description,
                folder,
                tags: vec!["beeftext".to_string()],
                case_mode: CaseMode::MatchTyped,
            }
        })
        .collect())
}

pub fn export_meetin_json(
    phrases: &[PhraseRecord],
    folders: &[FolderRecord],
) -> Result<String, FormatIoError> {
    let folder_names: HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();
    let doc = MeetinExportDoc {
        format: "meetin".to_string(),
        version: 1,
        folders: folders.iter().map(|f| f.name.clone()).collect(),
        phrases: phrases
            .iter()
            .filter(|p| p.deleted_at.is_none())
            .map(|p| MeetinExportPhrase {
                abbr: p.abbr.clone(),
                content: p.content.clone(),
                content_html: p.content_html.clone(),
                description: p.description.clone(),
                folder: p.folder_id.and_then(|fid| folder_names.get(&fid).cloned()),
                tags: p.tags.clone(),
                case_mode: p.case_mode,
            })
            .collect(),
    };
    Ok(serde_json::to_string_pretty(&doc)?)
}

pub fn export_csv(
    phrases: &[PhraseRecord],
    folders: &[FolderRecord],
) -> Result<String, FormatIoError> {
    let folder_names: HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record([
        "abbr",
        "content",
        "description",
        "folder",
        "tags",
        "case_mode",
        "use_count",
    ])?;

    for p in phrases.iter().filter(|p| p.deleted_at.is_none()) {
        let folder = p
            .folder_id
            .and_then(|fid| folder_names.get(&fid).cloned())
            .unwrap_or_default();
        let tags = p.tags.join(",");
        let mode = match p.case_mode {
            CaseMode::MatchTyped => "match_typed",
            CaseMode::KeepOriginal => "keep_original",
            CaseMode::ExactMatch => "exact_match",
        };
        wtr.write_record([
            p.abbr.as_str(),
            p.content.as_str(),
            p.description.as_str(),
            folder.as_str(),
            tags.as_str(),
            mode,
            &p.use_count.to_string(),
        ])?;
    }
    let bytes = wtr
        .into_inner()
        .map_err(|e| FormatIoError::Csv(e.into_error().into()))?;
    let body = String::from_utf8_lossy(&bytes);
    Ok(format!("\u{FEFF}{}", body))
}

pub fn export_ahk(phrases: &[PhraseRecord]) -> String {
    let mut out = String::from("; Meetin Dışa Aktarım (AutoHotkey)\n");
    for p in phrases.iter().filter(|p| p.deleted_at.is_none() && p.enabled) {
        let opts = match p.case_mode {
            CaseMode::KeepOriginal => "*C",
            _ => "*",
        };
        out.push_str(&format!(
            ":{}:{}::{}\n",
            opts,
            p.abbr,
            escape_ahk(&p.content)
        ));
    }
    out
}

/// Dosya uzantısına göre `.ahk`, `.txt`, `.json`, `.meetin` veya `.mee` içeriğini ayrıştırır.
pub fn parse_import_file(
    filename: &str,
    raw: &[u8],
) -> Result<Vec<ImportedPhraseItem>, FormatIoError> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("ahk") {
        Ok(parse_ahk_hotstrings(raw))
    } else if ext.eq_ignore_ascii_case("txt") {
        Ok(parse_txt_blocks(raw))
    } else if ext.eq_ignore_ascii_case("json")
        || ext.eq_ignore_ascii_case("meetin")
        || ext.eq_ignore_ascii_case("mee")
    {
        if looks_like_zip(raw) {
            Ok(import_mee_archive(raw)?)
        } else {
            parse_beeftext_or_meetin_json(raw)
        }
    } else {
        Err(FormatIoError::UnsupportedFormat(ext.to_string()))
    }
}

fn suggest_unique_imported_abbr(
    base: &str,
    used_norms: &HashSet<String>,
    fold_dotted_i: bool,
) -> String {
    let mut n = 2usize;
    loop {
        let candidate = format!("{}{}", base, n);
        let norm = normalize_abbr(&candidate, fold_dotted_i);
        if !used_norms.contains(&norm) {
            return candidate;
        }
        n += 1;
    }
}

/// Ayrıştırılmış veya toplu girilmiş ifadeleri çakışma stratejisiyle veritabanına aktarır.
pub fn import_items(
    storage: &Storage,
    items: &[ImportedPhraseItem],
    strategy: ConflictStrategy,
    fold_dotted_i: bool,
) -> Result<CsvImportSummary, FormatIoError> {
    let existing = storage.list_phrases(false)?;
    let mut existing_by_norm: HashMap<String, i64> = HashMap::new();
    let mut used_norms: HashSet<String> = HashSet::new();
    for p in existing {
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

    for item in items {
        let abbr = item.abbr.trim();
        if abbr.is_empty() || item.content.is_empty() {
            continue;
        }
        summary.total_valid += 1;
        let folder_id = match item.folder.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => Some(storage.get_or_create_folder(name)?),
            None => None,
        };
        let norm = normalize_abbr(abbr, fold_dotted_i);
        if let Some(existing_id) = existing_by_norm.get(&norm).copied() {
            summary.conflicts.push(abbr.to_string());
            match strategy {
                ConflictStrategy::Skip => {
                    summary.skipped += 1;
                }
                ConflictStrategy::Overwrite => {
                    storage.update_phrase(&UpdatePhrase {
                        id: existing_id,
                        abbr: abbr.to_string(),
                        content: item.content.clone(),
                        content_html: item.content_html.clone(),
                        description: item.description.clone(),
                        folder_id,
                        folder_ids: folder_id.into_iter().collect(),
                        case_mode: item.case_mode,
                        shortcut: None,
                        enabled: true,
                        tags: item.tags.clone(),
                    })?;
                    summary.overwritten += 1;
                }
                ConflictStrategy::SuggestNew => {
                    let new_abbr =
                        suggest_unique_imported_abbr(abbr, &used_norms, fold_dotted_i);
                    let new_norm = normalize_abbr(&new_abbr, fold_dotted_i);
                    used_norms.insert(new_norm);
                    let created = storage.create_phrase(&NewPhrase {
                        abbr: new_abbr,
                        content: item.content.clone(),
                        content_html: item.content_html.clone(),
                        description: item.description.clone(),
                        folder_id,
                        folder_ids: folder_id.into_iter().collect(),
                        case_mode: item.case_mode,
                        shortcut: None,
                        tags: item.tags.clone(),
                    })?;
                    existing_by_norm.insert(normalize_abbr(&created.abbr, fold_dotted_i), created.id);
                    summary.renamed += 1;
                    summary.imported += 1;
                }
            }
        } else {
            let created = storage.create_phrase(&NewPhrase {
                abbr: abbr.to_string(),
                content: item.content.clone(),
                content_html: item.content_html.clone(),
                description: item.description.clone(),
                folder_id,
                folder_ids: folder_id.into_iter().collect(),
                case_mode: item.case_mode,
                shortcut: None,
                tags: item.tags.clone(),
            })?;
            existing_by_norm.insert(norm.clone(), created.id);
            used_norms.insert(norm);
            summary.imported += 1;
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ahk_single_and_multiline_hotstrings() {
        let sample = r#"; yorum
:*:mrb::Merhaba{!}
:C:İmza::
(
Saygılarımla,
Kullanıcı
)
"#
        .as_bytes();
        let items = parse_ahk_hotstrings(sample);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].abbr, "mrb");
        assert_eq!(items[0].content, "Merhaba{!}");
        assert_eq!(items[1].abbr, "İmza");
        assert_eq!(items[1].case_mode, CaseMode::KeepOriginal);
        assert!(items[1].content.contains("Saygılarımla,\nKullanıcı"));
    }

    #[test]
    fn parses_txt_blocks_with_turkish_initials() {
        let sample = "İyi toplu ürün\n\nÇok güzel günler dilerim".as_bytes();
        let items = parse_txt_blocks(sample);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].abbr, "İTÜ");
        assert_eq!(items[1].abbr, "ÇGGD");
    }

    #[test]
    fn meetin_json_round_trip_and_beeftext_import() {
        let folders = vec![FolderRecord {
            id: 7,
            parent_id: None,
            name: "İş".to_string(),
            color: None,
            sort_order: 1,
        }];
        let phrases = vec![PhraseRecord {
            id: 1,
            folder_id: Some(7),
            folder_ids: vec![7],
            abbr: "selam".to_string(),
            content: "Merhaba ekip!".to_string(),
            content_html: Some("<b>Merhaba</b> ekip!".to_string()),
            description: "Karşılama".to_string(),
            case_mode: CaseMode::MatchTyped,
            shortcut: None,
            enabled: true,
            use_count: 4,
            last_used_at: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
            tags: vec!["ekip".to_string()],
        }];

        let json = export_meetin_json(&phrases, &folders).unwrap();
        let parsed = parse_beeftext_or_meetin_json(json.as_bytes()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].abbr, "selam");
        assert_eq!(parsed[0].folder.as_deref(), Some("İş"));
        assert_eq!(
            parsed[0].content_html.as_deref(),
            Some("<b>Merhaba</b> ekip!")
        );

        let beeftext = r#"{
            "groups": [{"uuid": "g1", "name": "Hızlı"}],
            "combos": [{"keyword": "bt1", "snippet": "Beeftext metni", "name": "Test", "group": "g1"}]
        }"#;
        let bt_items = parse_beeftext_or_meetin_json(beeftext.as_bytes()).unwrap();
        assert_eq!(bt_items.len(), 1);
        assert_eq!(bt_items[0].abbr, "bt1");
        assert_eq!(bt_items[0].folder.as_deref(), Some("Hızlı"));
    }

    #[test]
    fn exports_csv_with_bom_and_ahk() {
        let phrases = vec![PhraseRecord {
            id: 1,
            folder_id: None,
            folder_ids: vec![],
            abbr: "tş".to_string(),
            content: "Teşekkürler\nİyi çalışmalar".to_string(),
            content_html: None,
            description: String::new(),
            case_mode: CaseMode::KeepOriginal,
            shortcut: None,
            enabled: true,
            use_count: 2,
            last_used_at: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
            tags: vec![],
        }];
        let csv = export_csv(&phrases, &[]).unwrap();
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("tş"));

        let ahk = export_ahk(&phrases);
        assert!(ahk.contains(":*C:tş::Teşekkürler`nİyi çalışmalar"));
    }
}
