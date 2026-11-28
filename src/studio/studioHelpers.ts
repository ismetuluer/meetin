import { t } from "../i18n";
import type { ExportFormat } from "./ExportDialog";
import type {
  CaseMode,
  ClipboardHistoryRecord,
  FolderRecord,
  PhraseRecord,
} from "./types";

/** Stüdyo ortak tipleri, sabitleri ve saf yardımcıları (R4.2: StudioView.tsx'ten ayrıldı). */

export type SidebarView =
  | { kind: "all" }
  | { kind: "recent" }
  | { kind: "clipboard" }
  | { kind: "stats" }
  | { kind: "trash" }
  | { kind: "folder"; folderId: number };

export type SortMode = "updated" | "uses" | "abbr" | "recent";
export type PageMode = "studio" | "settings" | "help";

export interface EditorDraft {
  id: number | null;
  abbr: string;
  content: string;
  content_html: string | null;
  description: string;
  folder_id: number | null;
  folder_ids: number[];
  case_mode: CaseMode;
  shortcut: string;
  enabled: boolean;
  tags: string[];
}

export const EMPTY_DRAFT: EditorDraft = {
  id: null,
  abbr: "",
  content: "",
  content_html: null,
  description: "",
  folder_id: null,
  folder_ids: [],
  case_mode: "keep_original",
  shortcut: "",
  enabled: true,
  tags: [],
};

export const FOLDER_COLORS = ["c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8"] as const;

/** İşlem bildiriminin ekranda kalma süresi (R3.2) */
export const NOTICE_MS = 2600;

export type ContextMenuState =
  | { kind: "phrase"; x: number; y: number; phrase: PhraseRecord; ids: number[] }
  | { kind: "folder"; x: number; y: number; folder: FolderRecord }
  | null;

export const CASE_MODE_SYMBOLS: Record<
  CaseMode,
  {
    symbol: string;
    labelKey: "caseMode.match_typed" | "caseMode.keep_original" | "caseMode.exact_match";
    descKey: "caseMode.match_typedDesc" | "caseMode.keep_originalDesc" | "caseMode.exact_matchDesc";
  }
> = {
  match_typed: {
    symbol: "Aa",
    labelKey: "caseMode.match_typed",
    descKey: "caseMode.match_typedDesc",
  },
  keep_original: {
    symbol: "A=",
    labelKey: "caseMode.keep_original",
    descKey: "caseMode.keep_originalDesc",
  },
  exact_match: {
    symbol: "AA",
    labelKey: "caseMode.exact_match",
    descKey: "caseMode.exact_matchDesc",
  },
};

export function deriveAutoDescription(content: string): string {
  const firstLine =
    content
      .split(/\r?\n/)
      .map((l) => l.trim())
      .find(Boolean) ?? "";
  return firstLine.slice(0, 80);
}

const UI_PREFS_KEY = "meetin:ui_prefs";

export interface UiPrefs {
  sidebarCollapsed?: boolean;
  view?: SidebarView;
  sortMode?: SortMode;
  selectedPhraseId?: number | null;
}

export function loadUiPrefs(): UiPrefs {
  try {
    const raw = window.localStorage.getItem(UI_PREFS_KEY);
    return raw ? (JSON.parse(raw) as UiPrefs) : {};
  } catch {
    return {};
  }
}

/** Arayüz tercihleri yalnız kolaylıktır; depolama kapalıysa yazılmaz, akış sürer. */
export function saveUiPrefs(prefs: UiPrefs): void {
  try {
    window.localStorage.setItem(UI_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // yok say
  }
}

export const VARIABLE_SNIPPETS = [
  { label: "{tarih}", snippet: "{tarih}" },
  { label: "{tarih:uzun}", snippet: "{tarih:uzun}" },
  { label: "{saat}", snippet: "{saat}" },
  { label: "{pano}", snippet: "{pano}" },
  { label: "{imleç}", snippet: "{imleç}" },
  { label: "{alan:…}", snippet: "{alan:Ad Soyad}" },
  { label: "{ifade:…}", snippet: "{ifade:KISALTMA}" },
];

/** Yazı biçimli dışa aktarma dosyasının MIME türü. */
export function exportMime(format: ExportFormat): string {
  return format === "csv" ? "text/csv;charset=utf-8" : "text/plain;charset=utf-8";
}

/** Dosya adı için güvenli parça: boşluklar tire, Windows'ta yasak karakterler atılır. */
export function exportFileBase(name: string): string {
  const base = name
    .toLocaleLowerCase("tr-TR")
    .replace(/[\\/:*?"<>|]/g, "")
    .trim()
    .replace(/\s+/g, "-");
  return base || "meetin-liste";
}

/** Klavye kısayolu yazı alanındaysa (girdi, düzenleyici) liste kısayolları çalışmaz. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName);
}

export function formatRelativeDate(iso: string | null): string {
  if (!iso) return "—";
  const ts = Date.parse(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  if (Number.isNaN(ts)) return iso.slice(0, 10);
  const diffDays = Math.floor((Date.now() - ts) / 86_400_000);
  if (diffDays <= 0) return t("studio.today");
  if (diffDays === 1) return t("studio.yesterday");
  return t("studio.daysAgo", { count: diffDays });
}

/** Sürüm zamanı: göreli gün ve yerel saat (aynı gün birden çok sürüm ayırt edilsin). */
export function formatVersionTime(iso: string): string {
  const ts = Date.parse(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  if (Number.isNaN(ts)) return iso;
  const time = new Date(ts).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
  return `${formatRelativeDate(iso)} · ${time}`;
}

/** Sürüklenen satırların kimlikleri: çoklu seçim JSON'u, yoksa tek kimlik. */
export function extractDraggedIds(dt: DataTransfer): number[] {
  const multi = dt.getData("application/x-meetin-phrase-ids");
  if (multi) {
    try {
      const parsed = JSON.parse(multi) as number[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed.filter((n) => Number.isFinite(n) && n > 0);
      }
    } catch {
      // yok say
    }
  }
  const single = Number(dt.getData("text/plain"));
  return Number.isFinite(single) && single > 0 ? [single] : [];
}

/** Orta sütunda gösterilen ifadeler: görünüm, arama ve sıralama uygulanmış. */
export function filterDisplayedPhrases(
  activePhrases: PhraseRecord[],
  trashPhrases: PhraseRecord[],
  view: SidebarView,
  query: string,
  sortMode: SortMode,
): PhraseRecord[] {
  const q = query.trim().toLocaleLowerCase("tr-TR");
  let base = view.kind === "trash" ? trashPhrases : activePhrases;

  if (view.kind === "folder") {
    base = base.filter(
      (p) =>
        p.folder_ids?.includes(view.folderId) ||
        p.folder_id === view.folderId,
    );
  } else if (view.kind === "recent") {
    base = base.filter((p) => p.last_used_at !== null || p.use_count > 0);
  }

  if (q) {
    base = base.filter(
      (p) =>
        p.abbr.toLocaleLowerCase("tr-TR").includes(q) ||
        p.content.toLocaleLowerCase("tr-TR").includes(q) ||
        p.description.toLocaleLowerCase("tr-TR").includes(q) ||
        p.tags.some((tag) => tag.toLocaleLowerCase("tr-TR").includes(q)),
    );
  }

  const sorted = [...base];
  const effSort: SortMode = view.kind === "recent" ? "recent" : sortMode;
  sorted.sort((a, b) => {
    if (effSort === "uses") return b.use_count - a.use_count;
    if (effSort === "abbr") return a.abbr.localeCompare(b.abbr, "tr-TR");
    if (effSort === "recent") {
      return (b.last_used_at ?? "").localeCompare(a.last_used_at ?? "");
    }
    return b.updated_at.localeCompare(a.updated_at);
  });
  return sorted;
}

/** Pano geçmişinde arama (içerik ya da kaynak uygulama). */
export function filterClipboardItems(
  clipboardItems: ClipboardHistoryRecord[],
  query: string,
): ClipboardHistoryRecord[] {
  const q = query.trim().toLocaleLowerCase("tr-TR");
  if (!q) return clipboardItems;
  return clipboardItems.filter(
    (c) =>
      c.content.toLocaleLowerCase("tr-TR").includes(q) ||
      (c.source_process ?? "").toLocaleLowerCase("tr-TR").includes(q),
  );
}

/** Orta sütun başlığı: görünüm adı ya da klasör adı. */
export function viewTitle(view: SidebarView, folders: FolderRecord[]): string {
  if (view.kind === "all") return t("studio.viewAll");
  if (view.kind === "recent") return t("studio.viewRecent");
  if (view.kind === "clipboard") return t("studio.viewClipboard");
  if (view.kind === "stats") return t("studio.viewStats");
  if (view.kind === "trash") return t("studio.viewTrash");
  return folders.find((f) => f.id === view.folderId)?.name ?? t("studio.foldersTitle");
}
