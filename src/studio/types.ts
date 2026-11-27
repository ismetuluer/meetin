export type CaseMode = "match_typed" | "keep_original" | "exact_match";

export interface PhraseRecord {
  id: number;
  folder_id: number | null;
  folder_ids: number[];
  abbr: string;
  content: string;
  content_html: string | null;
  description: string;
  case_mode: CaseMode;
  shortcut: string | null;
  enabled: boolean;
  use_count: number;
  last_used_at: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  tags: string[];
}

export interface FolderRecord {
  id: number;
  parent_id: number | null;
  name: string;
  color: string | null;
  sort_order: number;
}

export interface EngineStatus {
  paused: boolean;
  fold_dotted_i: boolean;
  active_phrase_count: number;
  data_dir: string;
  version: string;
}

export type ConflictStrategy = "skip" | "overwrite" | "suggest_new";

export interface CsvTablePreview {
  detected_encoding: string;
  detected_delimiter: string;
  headers: string[];
  sample_rows: string[][];
  total_rows: number;
}

export interface CsvColumnMapping {
  abbr_col: number;
  content_col: number;
  description_col: number | null;
  folder_col: number | null;
  tags_col: number | null;
  has_header: boolean;
}

export interface CsvImportSummary {
  total_valid: number;
  imported: number;
  overwritten: number;
  skipped: number;
  renamed: number;
  conflicts: string[];
}

export type AppRuleKind = "only" | "exclude";

export interface AppRule {
  kind: AppRuleKind;
  process_name: string;
  title_contains: string | null;
}

export interface PhraseVersionRecord {
  id: number;
  phrase_id: number;
  abbr: string;
  content: string;
  content_html: string | null;
  description: string;
  created_at: string;
}

export interface ClipboardHistoryRecord {
  id: number;
  content: string;
  content_html: string | null;
  copied_at: string;
  source_process: string | null;
}

export interface TopPhraseStat {
  id: number;
  abbr: string;
  use_count: number;
  last_used_at: string | null;
}

export interface UsageStats {
  today_count: number;
  week_count: number;
  total_count: number;
  saved_chars: number;
  saved_minutes: number;
  active_phrases: number;
  top_phrases: TopPhraseStat[];
}

export interface ImportedPhraseItem {
  abbr: string;
  content: string;
  content_html: string | null;
  description: string;
  folder: string | null;
  tags: string[];
  case_mode: CaseMode;
}

export interface TriggerConfig {
  space: boolean;
  enter: boolean;
  tab: boolean;
  punctuation: boolean;
}

export interface ShortcutConfigDto {
  pause: string;
  search: string;
  quick_add: string;
  studio: string;
  toggle_mascot: string;
}

export type AbbrSuggestMode = "initials" | "first_word" | "last_word" | "tag";

export interface RunningAppInfo {
  process_name: string;
  window_title: string;
}

export interface AppSettingsDto {
  trigger_config: TriggerConfig;
  suggest_enabled: boolean;
  excluded_processes: string[];
  mascot_sound: boolean;
  mascot_size: "small" | "normal" | "large";
  user_name: string;
  onboarding_done: boolean;
  shortcuts: ShortcutConfigDto;
  language: string;
  clipboard_enabled: boolean;
  clipboard_limit: number;
  abbr_suggest_mode: AbbrSuggestMode;
  shift_f3_enabled: boolean;
  sound_baa: boolean;
  sound_throw: boolean;
  sound_impact: boolean;
  auto_check_updates: boolean;
}

export interface BackupFileInfo {
  filename: string;
  path: string;
  size_bytes: number;
  modified_at: string;
}

export interface SuggestionItem {
  phrase_id: number;
  abbr: string;
  preview: string;
  typed_len: number;
}

export interface StartupStatus {
  run_at_login: boolean;
  admin_task_enabled: boolean;
  is_elevated: boolean;
  is_portable: boolean;
}

/** Güncelleme denetimi sonucu; paket imzası kurulum sırasında eklenti tarafından doğrulanır. */
export interface UpdateCheckResult {
  current_version: string;
  /** Denetim yapılmadıysa null. */
  latest_version: string | null;
  update_available: boolean;
  notes: string;
  /** Elle indirme adresi (portable ZIP ya da yayın sayfası). */
  download_url: string;
  is_portable: boolean;
  webview2_installed: boolean;
  can_auto_install: boolean;
}

export interface UpdateInstallResult {
  version: string;
}
