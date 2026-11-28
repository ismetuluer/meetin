import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type FormEvent,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t, useLanguage, type TranslationKey } from "../i18n";
import { suggestAbbreviation } from "../island/abbrSuggest";
import { KbdGroup, ShortcutField } from "../design/Kbd";
import { StackPanel } from "../design/StackPanel";
import { TagInput } from "../design/TagInput";
import { MeetinSheep } from "../mascot/MeetinSheep";
import { BulkAddModal } from "./BulkAddModal";
import { CommandPalette, type PaletteCommand } from "./CommandPalette";
import { CsvImportModal } from "./CsvImportModal";
import { HelpView } from "./HelpView";
import { OnboardingModal } from "./OnboardingModal";
import { RichEditor } from "./RichEditor";
import { SettingsView } from "./SettingsView";
import { plainTextToRichHtml } from "./richHtml";
import {
  buildEmbeddedImageTag,
  extractEmbeddedImages,
  isSafeEmbeddedImageDataUrl,
  isSupportedImageFile,
  MAX_EMBEDDED_IMAGE_BYTES,
  removeEmbeddedImageAt,
} from "./richImages";
import {
  actionIds,
  clickRow,
  EMPTY_SELECTION,
  pruneSelection,
  type ListSelection,
} from "./listSelection";
import { captureShortcutCombo } from "./shortcutCapture";
import type {
  AbbrSuggestMode,
  AppRule,
  AppSettingsDto,
  CaseMode,
  ClipboardHistoryRecord,
  CsvImportSummary,
  EngineStatus,
  FolderRecord,
  ImportedPhraseItem,
  PhraseRecord,
  PhraseVersionRecord,
  RunningAppInfo,
  UsageStats,
} from "./types";
import "./studio.css";

type SidebarView =
  | { kind: "all" }
  | { kind: "recent" }
  | { kind: "clipboard" }
  | { kind: "stats" }
  | { kind: "trash" }
  | { kind: "folder"; folderId: number };

type SortMode = "updated" | "uses" | "abbr" | "recent";
type PageMode = "studio" | "settings" | "help";

interface EditorDraft {
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

const EMPTY_DRAFT: EditorDraft = {
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

const FOLDER_COLORS = ["c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8"] as const;

type ContextMenuState =
  | { kind: "phrase"; x: number; y: number; phrase: PhraseRecord; ids: number[] }
  | { kind: "folder"; x: number; y: number; folder: FolderRecord }
  | null;

const CASE_MODE_SYMBOLS: Record<
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

function deriveAutoDescription(content: string): string {
  const firstLine =
    content
      .split(/\r?\n/)
      .map((l) => l.trim())
      .find(Boolean) ?? "";
  return firstLine.slice(0, 80);
}

const UI_PREFS_KEY = "meetin:ui_prefs";

interface UiPrefs {
  sidebarCollapsed?: boolean;
  view?: SidebarView;
  sortMode?: SortMode;
  selectedPhraseId?: number | null;
}

function loadUiPrefs(): UiPrefs {
  try {
    const raw = window.localStorage.getItem(UI_PREFS_KEY);
    return raw ? (JSON.parse(raw) as UiPrefs) : {};
  } catch {
    return {};
  }
}

const VARIABLE_SNIPPETS = [
  { label: "{tarih}", snippet: "{tarih}" },
  { label: "{tarih:uzun}", snippet: "{tarih:uzun}" },
  { label: "{saat}", snippet: "{saat}" },
  { label: "{pano}", snippet: "{pano}" },
  { label: "{imleç}", snippet: "{imleç}" },
  { label: "{alan:…}", snippet: "{alan:Ad Soyad}" },
  { label: "{ifade:…}", snippet: "{ifade:KISALTMA}" },
];

const VAR_LABEL_KEYS: Record<string, TranslationKey> = {
  "{tarih}": "var.date",
  "{tarih:uzun}": "var.dateLong",
  "{saat}": "var.time",
  "{pano}": "var.clipboard",
  "{imleç}": "var.cursor",
};

function getVarLabel(token: string): string {
  const key = VAR_LABEL_KEYS[token];
  return key ? t(key) : token.slice(1, -1);
}

/** Klavye kısayolu yazı alanındaysa (girdi, düzenleyici) liste kısayolları çalışmaz. */
function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName);
}

function formatPreviewLine(content: string): string {
  return content
    .replace(/\{([^}]+)\}/g, (full) => `[${getVarLabel(full)}]`)
    .replace(/\s+/g, " ")
    .trim();
}

function formatRelativeDate(iso: string | null): string {
  if (!iso) return "—";
  const ts = Date.parse(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  if (Number.isNaN(ts)) return iso.slice(0, 10);
  const diffDays = Math.floor((Date.now() - ts) / 86_400_000);
  if (diffDays <= 0) return t("studio.today");
  if (diffDays === 1) return t("studio.yesterday");
  return t("studio.daysAgo", { count: diffDays });
}

/** Sürüm zamanı: göreli gün ve yerel saat (aynı gün birden çok sürüm ayırt edilsin). */
function formatVersionTime(iso: string): string {
  const ts = Date.parse(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  if (Number.isNaN(ts)) return iso;
  const time = new Date(ts).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
  return `${formatRelativeDate(iso)} · ${time}`;
}

function renderContentSegments(text: string): { isVar: boolean; value: string }[] {
  const parts: { isVar: boolean; value: string }[] = [];
  const regex = /(\{[^}]+\})/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = regex.exec(text)) !== null) {
    if (m.index > last) {
      parts.push({ isVar: false, value: text.slice(last, m.index) });
    }
    parts.push({ isVar: true, value: getVarLabel(m[0]) });
    last = m.index + m[0].length;
  }
  if (last < text.length) {
    parts.push({ isVar: false, value: text.slice(last) });
  }
  return parts;
}

export function StudioView() {
  const lang = useLanguage();
  const initialPrefs = useMemo(() => loadUiPrefs(), []);
  const [pageMode, setPageMode] = useState<PageMode>("studio");
  const [settingsInitialSection, setSettingsInitialSection] = useState<
    "appearance" | "update"
  >("appearance");
  const [availableUpdateVersion, setAvailableUpdateVersion] = useState<
    string | null
  >(null);
  const [phrases, setPhrases] = useState<PhraseRecord[]>([]);
  const [folders, setFolders] = useState<FolderRecord[]>([]);
  const [clipboardItems, setClipboardItems] = useState<ClipboardHistoryRecord[]>([]);
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [abbrSuggestMode, setAbbrSuggestMode] = useState<AbbrSuggestMode>("initials");

  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(
    Boolean(initialPrefs.sidebarCollapsed),
  );
  const [view, setView] = useState<SidebarView>(initialPrefs.view ?? { kind: "all" });
  const [sortMode, setSortMode] = useState<SortMode>(initialPrefs.sortMode ?? "updated");
  const [query, setQuery] = useState("");
  const [newFolderName, setNewFolderName] = useState("");
  const [showFolderInput, setShowFolderInput] = useState(false);
  const [dropFolderId, setDropFolderId] = useState<number | null>(null);
  const [dropTrash, setDropTrash] = useState(false);
  const [selection, setSelection] = useState<ListSelection>(EMPTY_SELECTION);
  const [swipedId, setSwipedId] = useState<number | null>(null);
  const [showListExportMenu, setShowListExportMenu] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
  const [renamingFolderId, setRenamingFolderId] = useState<number | null>(null);
  const [renameFolderDraft, setRenameFolderDraft] = useState("");

  const [draft, setDraft] = useState<EditorDraft | null>(null);
  const [descriptionManual, setDescriptionManual] = useState(false);
  const [autoSaveState, setAutoSaveState] = useState<"idle" | "saving" | "saved">("idle");
  const [appRules, setAppRules] = useState<AppRule[]>([]);
  const [runningApps, setRunningApps] = useState<RunningAppInfo[]>([]);
  const [newRuleProc, setNewRuleProc] = useState("");
  const [newRuleKind, setNewRuleKind] = useState<AppRule["kind"]>("only");
  const [showRuleAdd, setShowRuleAdd] = useState(false);
  const [showCaseMenu, setShowCaseMenu] = useState(false);

  const [versions, setVersions] = useState<PhraseVersionRecord[]>([]);
  const [showHistoryDrawer, setShowHistoryDrawer] = useState(false);
  const [editorSessionId, setEditorSessionId] = useState(0);
  const [showMoreMenu, setShowMoreMenu] = useState(false);

  const [showCsv, setShowCsv] = useState(false);
  const [showBulkModal, setShowBulkModal] = useState(false);
  const [importPreview, setImportPreview] = useState<{
    items: ImportedPhraseItem[];
    filename: string;
  } | null>(null);
  const [showPalette, setShowPalette] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [userName, setUserName] = useState("");

  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const imageInputRef = useRef<HTMLInputElement | null>(null);
  const autoSelectedRef = useRef(false);
  const lastSavedSnapshotRef = useRef<string>("");
  const autoSaveTimerRef = useRef<number | null>(null);
  const savingInFlightRef = useRef(false);

  const draftSnapshot = useCallback((d: EditorDraft) => {
    return JSON.stringify({
      id: d.id,
      abbr: d.abbr.trim(),
      content: d.content,
      content_html: d.content_html,
      description: d.description.trim(),
      folder_ids: d.folder_ids,
      case_mode: d.case_mode,
      shortcut: d.shortcut.trim(),
      enabled: d.enabled,
      tags: d.tags,
    });
  }, []);

  const loadPhraseMeta = useCallback(async (phraseId: number) => {
    try {
      const [rules, vers] = await Promise.all([
        invoke<AppRule[]>("get_phrase_app_rules", { phraseId }),
        invoke<PhraseVersionRecord[]>("list_phrase_versions", { phraseId }),
      ]);
      setAppRules(rules);
      setVersions(vers);
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }, []);

  const startEdit = useCallback(
    (p: PhraseRecord) => {
      if (autoSaveTimerRef.current !== null) {
        window.clearTimeout(autoSaveTimerRef.current);
        autoSaveTimerRef.current = null;
      }
      setError(null);
      setNotice(null);
      setShowHistoryDrawer(false);
      setShowRuleAdd(false);
      setShowCaseMenu(false);
      setAutoSaveState("idle");
      const folderIds =
        p.folder_ids && p.folder_ids.length > 0
          ? [...p.folder_ids]
          : p.folder_id !== null
            ? [p.folder_id]
            : [];
      const nextDraft: EditorDraft = {
        id: p.id,
        abbr: p.abbr,
        content: p.content,
        content_html: p.content_html,
        description: p.description,
        folder_id: folderIds[0] ?? null,
        folder_ids: folderIds,
        case_mode: p.case_mode,
        shortcut: p.shortcut ?? "",
        enabled: p.enabled,
        tags: [...p.tags],
      };
      const autoDesc = deriveAutoDescription(p.content);
      setDescriptionManual(
        p.description.trim().length > 0 && p.description.trim() !== autoDesc,
      );
      lastSavedSnapshotRef.current = draftSnapshot(nextDraft);
      setEditorSessionId((n) => n + 1);
      setDraft(nextDraft);
      void loadPhraseMeta(p.id);
    },
    [draftSnapshot, loadPhraseMeta],
  );

  const loadAll = useCallback(async () => {
    try {
      const [allPhrases, folderList, st, clipList, usage] = await Promise.all([
        invoke<PhraseRecord[]>("list_phrases", { includeDeleted: true }),
        invoke<FolderRecord[]>("list_folders"),
        invoke<EngineStatus>("get_engine_status"),
        invoke<ClipboardHistoryRecord[]>("list_clipboard_history", { limit: 100 }),
        invoke<UsageStats>("get_usage_stats"),
      ]);
      setPhrases(allPhrases);
      setFolders(folderList);
      setStatus(st);
      setClipboardItems(clipList);
      setStats(usage);

      if (!autoSelectedRef.current) {
        autoSelectedRef.current = true;
        const prefId = initialPrefs.selectedPhraseId;
        const preferred =
          prefId !== null && prefId !== undefined
            ? allPhrases.find((p) => p.id === prefId && p.deleted_at === null)
            : undefined;
        const firstActive = preferred ?? allPhrases.find((p) => p.deleted_at === null);
        if (firstActive) {
          startEdit(firstActive);
        }
      }
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }, [initialPrefs.selectedPhraseId, startEdit]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        UI_PREFS_KEY,
        JSON.stringify({
          sidebarCollapsed,
          view,
          sortMode,
          selectedPhraseId: draft?.id ?? null,
        } satisfies UiPrefs),
      );
    } catch {
      // yok say
    }
  }, [sidebarCollapsed, view, sortMode, draft?.id]);

  useEffect(() => {
    void loadAll();
    void invoke<AppSettingsDto>("get_app_settings")
      .then((s) => {
        setUserName(s.user_name);
        setAbbrSuggestMode(s.abbr_suggest_mode ?? "initials");
        if (!s.onboarding_done) {
          setShowOnboarding(true);
        }
      })
      .catch(() => {});
    void invoke<import("./types").UpdateCheckResult>("get_update_env_info")
      .then((u) => {
        if (u.update_available && u.latest_version) {
          setAvailableUpdateVersion(u.latest_version);
        }
      })
      .catch(() => {});
    const unlistenUpdate = listen<string>("update-available", (ev) => {
      if (ev.payload) {
        setAvailableUpdateVersion(ev.payload);
      }
    });
    const unlistenStatus = listen<boolean>("engine-status-changed", () => {
      void loadAll();
    });
    const unlistenExpand = listen<string>("phrase-expanded", () => {
      void loadAll();
    });
    const unlistenRestored = listen<void>("library-restored", () => {
      setDraft(null);
      autoSelectedRef.current = false;
      void loadAll();
    });
    const unlistenDraft = listen<{
      abbr: string;
      content: string;
      description: string;
    }>("studio-new-draft", (ev) => {
      setPageMode("studio");
      setError(null);
      setNotice(null);
      setAppRules([]);
      setVersions([]);
      setEditorSessionId((n) => n + 1);
      setDraft({
        ...EMPTY_DRAFT,
        abbr: ev.payload.abbr,
        content: ev.payload.content,
        description: ev.payload.description,
      });
    });

    return () => {
      void unlistenUpdate.then((f) => f());
      void unlistenStatus.then((f) => f());
      void unlistenExpand.then((f) => f());
      void unlistenRestored.then((f) => f());
      void unlistenDraft.then((f) => f());
    };
  }, [loadAll]);

  const activePhrases = useMemo(
    () => phrases.filter((p) => p.deleted_at === null),
    [phrases],
  );
  const trashPhrases = useMemo(
    () => phrases.filter((p) => p.deleted_at !== null),
    [phrases],
  );

  const selectedPhrase = useMemo(
    () =>
      draft?.id !== null && draft?.id !== undefined
        ? phrases.find((p) => p.id === draft.id) ?? null
        : null,
    [phrases, draft],
  );

  const displayedPhrases = useMemo(() => {
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
  }, [activePhrases, trashPhrases, view, query, sortMode]);

  const displayedOrder = useMemo(() => displayedPhrases.map((p) => p.id), [displayedPhrases]);

  // Filtre, görünüm ya da silme sonrası görünmeyen satırlar seçimden düşer
  useEffect(() => {
    setSelection((prev) => pruneSelection(prev, displayedOrder));
  }, [displayedOrder]);

  const listSelectable =
    view.kind !== "clipboard" && view.kind !== "stats" && view.kind !== "trash";

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setShowPalette((prev) => !prev);
      } else if (e.key === "F1") {
        e.preventDefault();
        setPageMode((m) => (m === "help" ? "studio" : "help"));
      } else if (
        e.key === "Escape" &&
        pageMode === "help" &&
        !showPalette &&
        !showOnboarding
      ) {
        e.preventDefault();
        setPageMode("studio");
      } else if (pageMode === "studio" && !showPalette && !isEditableTarget(e.target)) {
        // Liste seçimi (R3.1): Ctrl+A hepsini seçer, Esc seçimi bırakır
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a" && listSelectable) {
          e.preventDefault();
          setSelection((prev) => ({ ...prev, ids: displayedOrder }));
        } else if (e.key === "Escape" && selection.ids.length > 0) {
          e.preventDefault();
          setSelection((prev) => ({ ...prev, ids: [] }));
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pageMode, showPalette, showOnboarding, listSelectable, displayedOrder, selection.ids.length]);

  const filteredClipboard = useMemo(() => {
    const q = query.trim().toLocaleLowerCase("tr-TR");
    if (!q) return clipboardItems;
    return clipboardItems.filter(
      (c) =>
        c.content.toLocaleLowerCase("tr-TR").includes(q) ||
        (c.source_process ?? "").toLocaleLowerCase("tr-TR").includes(q),
    );
  }, [clipboardItems, query]);

  const openNewDraft = useCallback(
    (prefillContent = "", prefillHtml: string | null = null) => {
      if (autoSaveTimerRef.current !== null) {
        window.clearTimeout(autoSaveTimerRef.current);
        autoSaveTimerRef.current = null;
      }
      setPageMode("studio");
      setError(null);
      setNotice(null);
      setShowHistoryDrawer(false);
      setShowCaseMenu(false);
      setAutoSaveState("idle");
      setAppRules([]);
      setVersions([]);
      const fids = view.kind === "folder" ? [view.folderId] : [];
      const activeFolderName =
        view.kind === "folder"
          ? folders.find((f) => f.id === view.folderId)?.name ?? null
          : null;
      const next: EditorDraft = {
        ...EMPTY_DRAFT,
        abbr: prefillContent ? suggestAbbreviation(prefillContent, abbrSuggestMode) : "",
        content: prefillContent,
        content_html: prefillHtml,
        description: prefillContent ? deriveAutoDescription(prefillContent) : "",
        folder_id: fids[0] ?? null,
        folder_ids: fids,
        tags: activeFolderName ? [activeFolderName] : [],
      };
      setDescriptionManual(false);
      lastSavedSnapshotRef.current = "";
      setEditorSessionId((n) => n + 1);
      setDraft(next);
    },
    [abbrSuggestMode, folders, view],
  );

  async function handleTogglePause() {
    try {
      await invoke<boolean>("toggle_pause");
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleCreateFolder(e: FormEvent) {
    e.preventDefault();
    const trimmed = newFolderName.trim();
    if (!trimmed) return;
    setError(null);
    try {
      const created = await invoke<FolderRecord>("create_folder", {
        name: trimmed,
        parentId: null,
        color: null,
      });
      setNewFolderName("");
      setShowFolderInput(false);
      setView({ kind: "folder", folderId: created.id });
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleDeleteFolder(id: number) {
    setError(null);
    try {
      await invoke("delete_folder", { id });
      if (view.kind === "folder" && view.folderId === id) {
        setView({ kind: "all" });
      }
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  const persistDraft = useCallback(
    async (target: EditorDraft, silent: boolean) => {
      if (!target.abbr.trim() || !target.content.trim()) {
        if (!silent) setError(t("error.requiredFields"));
        return;
      }
      if (savingInFlightRef.current) return;
      savingInFlightRef.current = true;
      if (silent) {
        setAutoSaveState("saving");
      } else {
        setError(null);
        setNotice(null);
      }
      try {
        let saved: PhraseRecord;
        if (target.id === null) {
          saved = await invoke<PhraseRecord>("create_phrase", {
            input: {
              abbr: target.abbr.trim(),
              content: target.content,
              content_html: target.content_html,
              description: target.description.trim(),
              folder_id: target.folder_ids[0] ?? target.folder_id,
              folder_ids: target.folder_ids,
              case_mode: target.case_mode,
              shortcut: target.shortcut.trim() || null,
              tags: target.tags,
            },
          });
        } else {
          saved = await invoke<PhraseRecord>("update_phrase", {
            input: {
              id: target.id,
              abbr: target.abbr.trim(),
              content: target.content,
              content_html: target.content_html,
              description: target.description.trim(),
              folder_id: target.folder_ids[0] ?? target.folder_id,
              folder_ids: target.folder_ids,
              case_mode: target.case_mode,
              shortcut: target.shortcut.trim() || null,
              enabled: target.enabled,
              tags: target.tags,
            },
          });
        }
        if (appRules.length > 0 || target.id !== null) {
          await invoke("set_phrase_app_rules", {
            phraseId: saved.id,
            rules: appRules,
          });
        }
        const savedSnap = draftSnapshot({
          ...target,
          id: saved.id,
          folder_id: saved.folder_id,
          folder_ids: saved.folder_ids ?? [],
        });
        lastSavedSnapshotRef.current = savedSnap;
        setDraft((prev) =>
          prev
            ? {
                ...prev,
                id: saved.id,
                folder_id: saved.folder_id,
                folder_ids: saved.folder_ids ?? prev.folder_ids,
              }
            : prev,
        );
        await loadAll();
        if (silent) {
          setError(null);
          setAutoSaveState("saved");
        } else {
          startEdit(saved);
          setNotice(t("studio.savedNotice", { abbr: saved.abbr }));
        }
      } catch (err) {
        setAutoSaveState("idle");
        setError(t("error.generic", { message: String(err) }));
      } finally {
        savingInFlightRef.current = false;
      }
    },
    [appRules, draftSnapshot, loadAll, startEdit],
  );

  // Otomatik kaydetme: yeni ifade ve düzenlemeler 650 ms sonra otomatik kaydedilir
  useEffect(() => {
    if (!draft || pageMode !== "studio") return;
    if (!draft.abbr.trim() || !draft.content.trim()) return;
    const snap = draftSnapshot(draft);
    if (snap === lastSavedSnapshotRef.current) return;

    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
    }
    autoSaveTimerRef.current = window.setTimeout(() => {
      void persistDraft(draft, true);
    }, 650);

    return () => {
      if (autoSaveTimerRef.current !== null) {
        window.clearTimeout(autoSaveTimerRef.current);
      }
    };
  }, [draft, draftSnapshot, pageMode, persistDraft]);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!draft) return;
    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
      autoSaveTimerRef.current = null;
    }
    await persistDraft(draft, false);
  }

  async function handleFolderDrop(folderId: number, ids: number[], copy: boolean) {
    setDropFolderId(null);
    setError(null);
    try {
      for (const phraseId of ids) {
        const updated = await invoke<PhraseRecord>("move_phrase_folder", {
          phraseId,
          targetFolderId: folderId,
          sourceFolderId: view.kind === "folder" ? view.folderId : null,
          copy,
        });
        if (draft?.id === updated.id) {
          startEdit(updated);
        }
      }
      await loadAll();
      setNotice(copy ? t("studio.copiedToFolder") : t("studio.movedToFolder"));
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleBulkDelete(ids: number[]) {
    setDropTrash(false);
    setSwipedId(null);
    setError(null);
    try {
      for (const id of ids) {
        await invoke("delete_phrase", { id });
        if (draft?.id === id) {
          setDraft(null);
        }
      }
      setSelection((prev) => ({ ...prev, ids: prev.ids.filter((id) => !ids.includes(id)) }));
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleExportSubset(
    phraseIds: number[],
    format: "meetin" | "csv" | "ahk",
    filenameBase = "meetin-liste",
  ) {
    setShowListExportMenu(false);
    setContextMenu(null);
    setError(null);
    try {
      let blob: Blob;
      const ext = format === "meetin" ? "mee" : format;
      if (format === "meetin") {
        const bytes = await invoke<number[]>("export_phrases_subset_mee", {
          phraseIds,
        });
        blob = new Blob([new Uint8Array(bytes)], { type: "application/zip" });
      } else {
        const text = await invoke<string>("export_phrases_subset", {
          phraseIds,
          format,
        });
        const mime =
          format === "csv"
            ? "text/csv;charset=utf-8"
            : "text/plain;charset=utf-8";
        blob = new Blob([text], { type: mime });
      }
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${filenameBase}.${ext}`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleUpdateFolder(
    id: number,
    name: string | null,
    color: string | null,
  ) {
    setError(null);
    try {
      await invoke("update_folder", { id, name, color });
      setRenamingFolderId(null);
      setRenameFolderDraft("");
      await loadAll();
      setNotice(
        name !== null ? t("studio.folderRenamed") : t("studio.folderColorUpdated"),
      );
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  function extractDraggedIds(dt: DataTransfer): number[] {
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

  /** Tık / Ctrl+tık / Shift+tık (R3.1, `listSelection.ts`) */
  function handleRowClick(p: PhraseRecord, e: ReactMouseEvent) {
    const r = clickRow(
      selection,
      displayedOrder,
      p.id,
      { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
      draft?.id ?? null,
    );
    setSelection(r.sel);
    if (r.open) {
      setSwipedId(null);
      startEdit(p);
    }
  }

  async function handleRemoveFromFolder(ids: number[], folderId: number) {
    setError(null);
    try {
      for (const id of ids) {
        const p = phrases.find((x) => x.id === id);
        if (!p) continue;
        const rem = (p.folder_ids ?? []).filter((fid) => fid !== folderId);
        await invoke("update_phrase", {
          input: {
            id: p.id,
            abbr: p.abbr,
            content: p.content,
            content_html: p.content_html,
            description: p.description,
            folder_id: rem[0] ?? null,
            folder_ids: rem,
            case_mode: p.case_mode,
            shortcut: p.shortcut,
            enabled: p.enabled,
            tags: p.tags,
          },
        });
      }
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleClipboardTransformCopy(
    content: string,
    mode: "original" | "upper" | "lower" | "title",
    label: string,
  ) {
    setError(null);
    try {
      await invoke<string>("transform_clipboard_text", {
        text: content,
        mode,
        pasteNow: false,
      });
      setNotice(t("studio.copiedNotice", { mode: label }));
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleSoftDelete(id: number) {
    setError(null);
    try {
      await invoke("delete_phrase", { id });
      if (draft?.id === id) {
        setDraft(null);
      }
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleRestorePhrase(id: number) {
    setError(null);
    try {
      await invoke("restore_phrase", { id });
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleHardDelete(id: number) {
    setError(null);
    try {
      await invoke("hard_delete_phrase", { id });
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleEmptyTrash() {
    setError(null);
    try {
      await invoke<number>("empty_trash");
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleClearClipboard() {
    setError(null);
    try {
      await invoke<number>("clear_clipboard_history");
      await loadAll();
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleNewFromClipboard() {
    setError(null);
    try {
      const clip = await invoke<string>("read_clipboard_text");
      openNewDraft(clip);
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function persistRules(nextRules: AppRule[]) {
    setAppRules(nextRules);
    if (draft?.id !== null && draft?.id !== undefined) {
      try {
        await invoke("set_phrase_app_rules", {
          phraseId: draft.id,
          rules: nextRules,
        });
      } catch (err) {
        setError(t("error.generic", { message: String(err) }));
      }
    }
  }

  async function handleRestoreVersion(versionId: number) {
    if (!draft || draft.id === null) return;
    setError(null);
    try {
      const restored = await invoke<PhraseRecord>("restore_phrase_version", {
        phraseId: draft.id,
        versionId,
      });
      await loadAll();
      startEdit(restored);
      setNotice(t("studio.versionRestored"));
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleMultiImportFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    setError(null);
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      const items = await invoke<ImportedPhraseItem[]>("parse_import_file_cmd", {
        filename: file.name,
        bytes,
      });
      setImportPreview({ items, filename: file.name });
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleExport(format: "meetin" | "csv" | "ahk") {
    setShowMoreMenu(false);
    setError(null);
    try {
      let blob: Blob;
      const ext = format === "meetin" ? "mee" : format;
      if (format === "meetin") {
        const bytes = await invoke<number[]>("export_mee_archive");
        blob = new Blob([new Uint8Array(bytes)], { type: "application/zip" });
      } else {
        const text = await invoke<string>("export_library", { format });
        const mime =
          format === "csv"
            ? "text/csv;charset=utf-8"
            : "text/plain;charset=utf-8";
        blob = new Blob([text], { type: mime });
      }
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `meetin-yedek.${ext}`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  function handleInsertImageFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file || !draft) return;
    setError(null);
    setNotice(null);

    if (!isSupportedImageFile(file)) {
      setError(t("studio.imageUnsupportedType"));
      return;
    }
    if (file.size > MAX_EMBEDDED_IMAGE_BYTES) {
      setError(t("studio.imageTooLarge"));
      return;
    }

    const reader = new FileReader();
    reader.onerror = () => {
      setError(t("studio.imageReadError"));
    };
    reader.onload = () => {
      const dataUrl = typeof reader.result === "string" ? reader.result : "";
      if (!isSafeEmbeddedImageDataUrl(dataUrl)) {
        setError(t("studio.imageUnsupportedType"));
        return;
      }
      const imgTag = buildEmbeddedImageTag(
        dataUrl,
        file.name || t("studio.imageAltFallback"),
      );
      setDraft((prev) => {
        if (!prev) return prev;
        const baseHtml =
          prev.content_html ?? (prev.content ? plainTextToRichHtml(prev.content) : "");
        const nextHtml = baseHtml ? `${baseHtml}<br>${imgTag}` : imgTag;
        const nextContent = prev.content.trim()
          ? prev.content
          : `[${t("studio.imageAltFallback")}: ${file.name}]`;
        return {
          ...prev,
          content: nextContent,
          content_html: nextHtml,
          description: descriptionManual
            ? prev.description
            : prev.description.trim() || deriveAutoDescription(nextContent),
        };
      });
      setNotice(t("studio.imageAddedNotice"));
    };
    reader.readAsDataURL(file);
  }

  const paletteCommands: PaletteCommand[] = useMemo(
    () => [
      {
        id: "cmd-new",
        label: t("palette.actionNew"),
        onRun: () => openNewDraft(),
      },
      {
        id: "cmd-bulk",
        label: t("palette.actionBulk"),
        onRun: () => setShowBulkModal(true),
      },
      {
        id: "cmd-clipboard",
        label: t("palette.actionClipboard"),
        onRun: () => {
          setPageMode("studio");
          setView({ kind: "clipboard" });
        },
      },
      {
        id: "cmd-stats",
        label: t("palette.actionStats"),
        onRun: () => {
          setPageMode("studio");
          setView({ kind: "stats" });
        },
      },
      {
        id: "cmd-settings",
        label: t("studio.settingsBtn"),
        onRun: () => {
          setSettingsInitialSection("appearance");
          setPageMode("settings");
        },
      },
      {
        id: "cmd-help",
        label: t("palette.actionHelp"),
        hint: "F1",
        onRun: () => setPageMode("help"),
      },
      {
        id: "cmd-pause",
        label: t("palette.actionTogglePause"),
        onRun: () => void handleTogglePause(),
      },
    ],
    [lang, openNewDraft],
  );

  function handleImportDone(summary: CsvImportSummary) {
    setShowBulkModal(false);
    setImportPreview(null);
    setNotice(
      t("csv.doneText", {
        imported: summary.imported,
        overwritten: summary.overwritten,
        renamed: summary.renamed,
        skipped: summary.skipped,
      }),
    );
    void loadAll();
  }

  const centerTitle = useMemo(() => {
    if (view.kind === "all") return t("studio.viewAll");
    if (view.kind === "recent") return t("studio.viewRecent");
    if (view.kind === "clipboard") return t("studio.viewClipboard");
    if (view.kind === "stats") return t("studio.viewStats");
    if (view.kind === "trash") return t("studio.viewTrash");
    return folders.find((f) => f.id === view.folderId)?.name ?? t("studio.foldersTitle");
  }, [lang, view, folders]);

  const contentSegments = useMemo(
    () => (draft ? renderContentSegments(draft.content) : []),
    [lang, draft?.content],
  );

  const embeddedImages = useMemo(
    () => extractEmbeddedImages(draft?.content_html ?? null),
    [draft?.content_html],
  );

  return (
    <div
      className="fluent-shell"
      onClick={() => {
        setShowMoreMenu(false);
        setShowCaseMenu(false);
        setShowListExportMenu(false);
        setContextMenu(null);
        setSwipedId(null);
      }}
    >
      <header className="fluent-topbar">
        <div className="topbar-left">
          <MeetinSheep anim="idle" size={26} />
          <span className="topbar-brand">{t("app.name")}</span>
          {pageMode === "settings" && (
            <span className="topbar-sub">{t("studio.settingsBtn")}</span>
          )}
          {pageMode === "help" && (
            <span className="topbar-sub">{t("studio.helpBtn")}</span>
          )}
          {pageMode !== "studio" ? (
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => setPageMode("studio")}
            >
              ← {t("settings.backToStudio")}
            </button>
          ) : (
            status && (
              <button
                type="button"
                className={`status-dot-pill ${status.paused ? "is-paused" : "is-active"}`}
                onClick={() => void handleTogglePause()}
                title={status.paused ? t("status.resume") : t("status.pause")}
              >
                <span className="dot" />
                <span>{status.paused ? t("status.paused") : t("status.active")}</span>
              </button>
            )
          )}
        </div>

        <div className="topbar-right" onClick={(e) => e.stopPropagation()}>
          <button
            type="button"
            className="command-search-pill"
            title={t("tip.paletteBtn")}
            onClick={() => setShowPalette(true)}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.5" />
              <path d="M11 11L14.5 14.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
            <span className="muted">{t("studio.searchOrCommand")}</span>
            <span className="command-search-kbd">
              <KbdGroup shortcut="Ctrl+K" />
            </span>
          </button>

          <div className="dropdown-wrap">
            <button
              type="button"
              className="icon-btn"
              title={t("studio.moreActions")}
              onClick={() => setShowMoreMenu((v) => !v)}
            >
              ⋯
            </button>
            {showMoreMenu && (
              <div className="dropdown-menu">
                <button
                  type="button"
                  onClick={() => {
                    setShowMoreMenu(false);
                    void handleNewFromClipboard();
                  }}
                >
                  {t("toolbar.fromClipboard")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowMoreMenu(false);
                    setShowBulkModal(true);
                  }}
                >
                  {t("studio.bulkAddBtn")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowMoreMenu(false);
                    setShowCsv(true);
                  }}
                >
                  {t("toolbar.importCsv")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowMoreMenu(false);
                    fileInputRef.current?.click();
                  }}
                >
                  {t("studio.importMulti")}
                </button>
                <div className="menu-divider" />
                <button type="button" onClick={() => void handleExport("meetin")}>
                  {t("studio.exportMeetin")}
                </button>
                <button type="button" onClick={() => void handleExport("csv")}>
                  {t("studio.exportCsv")}
                </button>
                <button type="button" onClick={() => void handleExport("ahk")}>
                  {t("studio.exportAhk")}
                </button>
              </div>
            )}
          </div>
          <input
            ref={fileInputRef}
            type="file"
            accept=".mee,.meetin,.ahk,.txt,.json"
            style={{ display: "none" }}
            onChange={(e) => void handleMultiImportFile(e)}
          />
        </div>
      </header>

      {error && <div className="banner-inline is-error">{error}</div>}
      {notice && <div className="banner-inline is-ok">{notice}</div>}
      {availableUpdateVersion && pageMode !== "settings" && (
        <div
          className="banner-inline is-ok"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 12,
          }}
        >
          <span>
            {t("studio.updateAvailableBanner", { ver: availableUpdateVersion })}
          </span>
          <div className="row-actions" style={{ alignItems: "center" }}>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() => {
                setSettingsInitialSection("update");
                setPageMode("settings");
              }}
            >
              {t("studio.openUpdateBtn")}
            </button>
            <button
              type="button"
              className="mini-icon-btn"
              onClick={() => setAvailableUpdateVersion(null)}
            >
              ×
            </button>
          </div>
        </div>
      )}

      {pageMode === "settings" ? (
        <SettingsView
          status={status}
          initialSection={settingsInitialSection}
          onRefresh={() => void loadAll()}
          onExport={handleExport}
          onReplayOnboarding={() => setShowOnboarding(true)}
        />
      ) : pageMode === "help" ? (
        <HelpView
          onOpenSettings={() => {
            setSettingsInitialSection("appearance");
            setPageMode("settings");
          }}
          onReplayOnboarding={() => setShowOnboarding(true)}
          onCreatePhrase={() => openNewDraft()}
        />
      ) : (
        <div className={`fluent-workspace ${sidebarCollapsed ? "is-sidebar-collapsed" : ""}`}>
          <aside className={`fluent-sidebar ${sidebarCollapsed ? "is-collapsed" : ""}`}>
            <div className="sidebar-scroll">
              <div className="sidebar-collapse-row">
                <button
                  type="button"
                  className="sidebar-toggle-btn"
                  aria-expanded={!sidebarCollapsed}
                  aria-label={sidebarCollapsed ? t("studio.expandSidebar") : t("studio.collapseSidebar")}
                  title={sidebarCollapsed ? t("studio.expandSidebar") : t("studio.collapseSidebar")}
                  onClick={() => setSidebarCollapsed((c) => !c)}
                >
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                    <path d="M10 3.5L5.5 8L10 12.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </button>
              </div>
              <div className="sidebar-group">
                <button
                  type="button"
                  title={t("studio.viewRecent")}
                  className={`nav-item ${view.kind === "recent" ? "is-active" : ""}`}
                  onClick={() => setView({ kind: "recent" })}
                >
                  <span className="nav-label">
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                      <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.4" />
                      <path d="M8 4.8V8.2L10.4 9.6" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                    </svg>
                    <span className="nav-text">{t("studio.viewRecent")}</span>
                  </span>
                </button>

                <button
                  type="button"
                  title={t("studio.viewClipboard")}
                  className={`nav-item ${view.kind === "clipboard" ? "is-active" : ""}`}
                  onClick={() => setView({ kind: "clipboard" })}
                >
                  <span className="nav-label">
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                      <rect x="4" y="3" width="8" height="10.5" rx="1.5" stroke="currentColor" strokeWidth="1.4" />
                      <path d="M6 2.5H10" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
                    </svg>
                    <span className="nav-text">{t("studio.viewClipboard")}</span>
                  </span>
                  <span className="nav-count">{clipboardItems.length}</span>
                </button>
              </div>

              <div className="sidebar-group">
                {!sidebarCollapsed && (
                  <div className="sidebar-section-head">
                    <span>{t("studio.foldersTitle")}</span>
                    <button
                      type="button"
                      className="mini-icon-btn"
                      title={t("tip.addFolder")}
                      onClick={() => setShowFolderInput((v) => !v)}
                    >
                      +
                    </button>
                  </div>
                )}

                {showFolderInput && !sidebarCollapsed && (
                  <form onSubmit={handleCreateFolder} className="inline-folder-form">
                    <input
                      className="input"
                      autoFocus
                      value={newFolderName}
                      onChange={(e) => setNewFolderName(e.target.value)}
                      placeholder={t("studio.newFolderPlaceholder")}
                    />
                  </form>
                )}

                <button
                  type="button"
                  title={t("studio.viewAll")}
                  className={`nav-item ${view.kind === "all" ? "is-active" : ""}`}
                  onClick={() => setView({ kind: "all" })}
                >
                  <span className="nav-label">
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                      <path d="M2 4.5C2 3.7 2.7 3 3.5 3H6.4L7.7 4.5H12.5C13.3 4.5 14 5.2 14 6V11.5C14 12.3 13.3 13 12.5 13H3.5C2.7 13 2 12.3 2 11.5V4.5Z" stroke="currentColor" strokeWidth="1.4" />
                    </svg>
                    <span className="nav-text">{t("studio.viewAll")}</span>
                  </span>
                  <span className="nav-count">{activePhrases.length}</span>
                </button>

                {folders.map((folder) => {
                  const count = activePhrases.filter(
                    (p) => p.folder_ids?.includes(folder.id) || p.folder_id === folder.id,
                  ).length;
                  const isActive =
                    view.kind === "folder" && view.folderId === folder.id;
                  const isDropTarget = dropFolderId === folder.id;
                  const colorClass = folder.color ? `folder-tint-${folder.color}` : "";
                  return (
                    <div
                      key={folder.id}
                      title={`${folder.name} (Sürükleyerek taşı, Ctrl+bırak ile kopyala; sağ tıkla: renk ve seçenekler)`}
                      className={`nav-item folder-item ${isActive ? "is-active" : ""} ${isDropTarget ? "is-drop-target" : ""}`}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setContextMenu({
                          kind: "folder",
                          x: Math.min(e.clientX, window.innerWidth - 230),
                          y: Math.min(e.clientY, window.innerHeight - 260),
                          folder,
                        });
                      }}
                      onDragOver={(e) => {
                        e.preventDefault();
                        e.dataTransfer.dropEffect = e.ctrlKey ? "copy" : "move";
                        if (dropFolderId !== folder.id) {
                          setDropFolderId(folder.id);
                        }
                      }}
                      onDragLeave={() => {
                        if (dropFolderId === folder.id) {
                          setDropFolderId(null);
                        }
                      }}
                      onDrop={(e) => {
                        e.preventDefault();
                        const ids = extractDraggedIds(e.dataTransfer);
                        if (ids.length > 0) {
                          void handleFolderDrop(folder.id, ids, e.ctrlKey);
                        } else {
                          setDropFolderId(null);
                        }
                      }}
                    >
                      {renamingFolderId === folder.id ? (
                        <form
                          className="inline-folder-form"
                          style={{ flex: 1 }}
                          onSubmit={(e) => {
                            e.preventDefault();
                            void handleUpdateFolder(folder.id, renameFolderDraft, null);
                          }}
                        >
                          <input
                            className="input"
                            autoFocus
                            value={renameFolderDraft}
                            onChange={(e) => setRenameFolderDraft(e.target.value)}
                            onBlur={() => setRenamingFolderId(null)}
                            onKeyDown={(e) => {
                              if (e.key === "Escape") setRenamingFolderId(null);
                            }}
                          />
                        </form>
                      ) : (
                        <>
                          <button
                            type="button"
                            className="folder-main-btn"
                            onClick={() =>
                              setView({ kind: "folder", folderId: folder.id })
                            }
                          >
                            <span className="nav-label">
                              <svg
                                width="15"
                                height="15"
                                viewBox="0 0 16 16"
                                fill="none"
                                className={colorClass}
                              >
                                <path
                                  d="M2 4.5C2 3.7 2.7 3 3.5 3H6.4L7.7 4.5H12.5C13.3 4.5 14 5.2 14 6V11.5C14 12.3 13.3 13 12.5 13H3.5C2.7 13 2 12.3 2 11.5V4.5Z"
                                  stroke="currentColor"
                                  fill={folder.color ? "currentColor" : "none"}
                                  fillOpacity={folder.color ? 0.22 : 0}
                                  strokeWidth="1.4"
                                />
                              </svg>
                              <span className="nav-text">{folder.name}</span>
                            </span>
                            <span className="nav-count">{count}</span>
                          </button>
                          {!sidebarCollapsed && (
                            <button
                              type="button"
                              className="folder-del-x"
                              title={t("studio.deleteFolder")}
                              onClick={() => void handleDeleteFolder(folder.id)}
                            >
                              ×
                            </button>
                          )}
                        </>
                      )}
                    </div>
                  );
                })}
              </div>

              <div className="sidebar-group">
                <button
                  type="button"
                  title={t("studio.trashDropHint")}
                  className={`nav-item ${view.kind === "trash" ? "is-active" : ""} ${dropTrash ? "is-trash-drop-target" : ""}`}
                  onClick={() => setView({ kind: "trash" })}
                  onDragOver={(e) => {
                    e.preventDefault();
                    e.dataTransfer.dropEffect = "move";
                    if (!dropTrash) setDropTrash(true);
                  }}
                  onDragLeave={() => {
                    if (dropTrash) setDropTrash(false);
                  }}
                  onDrop={(e) => {
                    e.preventDefault();
                    const ids = extractDraggedIds(e.dataTransfer);
                    if (ids.length > 0) {
                      void handleBulkDelete(ids);
                    } else {
                      setDropTrash(false);
                    }
                  }}
                >
                  <span className="nav-label">
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                      <path d="M3 4.5H13M6 4.5V3H10V4.5M5 4.5V12.5H11V4.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                    <span className="nav-text">{t("studio.viewTrash")}</span>
                  </span>
                  <span className="nav-count">{trashPhrases.length}</span>
                </button>
              </div>
            </div>

            <div className="sidebar-footer">
              {!sidebarCollapsed ? (
                <button
                  type="button"
                  className={`weekly-summary-card ${view.kind === "stats" ? "is-active" : ""}`}
                  title={t("tip.weeklyStats")}
                  onClick={() => setView({ kind: "stats" })}
                >
                  <span className="muted small">{t("studio.thisWeek")}</span>
                  <strong>
                    {t("studio.weeklyExpansions", {
                      count: stats?.week_count ?? 0,
                    })}
                  </strong>
                  <span className="muted small">
                    {t("studio.weeklyMinutes", {
                      minutes: Math.round(stats?.saved_minutes ?? 0),
                    })}
                  </span>
                </button>
              ) : (
                <button
                  type="button"
                  className={`nav-item ${view.kind === "stats" ? "is-active" : ""}`}
                  title={t("tip.weeklyStats")}
                  onClick={() => setView({ kind: "stats" })}
                >
                  <span className="nav-label">
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                      <path d="M3 13V9M8 13V5M13 13V2.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                    </svg>
                  </span>
                </button>
              )}

              <button
                type="button"
                className="nav-item"
                title={t("tip.openHelp")}
                onClick={() => setPageMode("help")}
              >
                <span className="nav-label">
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.4" />
                    <path d="M6.4 6.2C6.6 5.3 7.2 4.8 8 4.8C8.9 4.8 9.5 5.4 9.5 6.2C9.5 6.9 9 7.3 8.4 7.7C8 8 7.9 8.3 7.9 8.9" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                    <circle cx="8" cy="11.2" r="0.8" fill="currentColor" />
                  </svg>
                  <span className="nav-text">{t("studio.helpBtn")}</span>
                </span>
              </button>

              <button
                type="button"
                className="nav-item"
                title={t("tip.openSettings")}
                onClick={() => {
                  setSettingsInitialSection("appearance");
                  setPageMode("settings");
                }}
              >
                <span className="nav-label">
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <circle cx="8" cy="8" r="2.2" stroke="currentColor" strokeWidth="1.4" />
                    <path d="M8 2V3.4M8 12.6V14M2 8H3.4M12.6 8H14M3.8 3.8L4.8 4.8M11.2 11.2L12.2 12.2M12.2 3.8L11.2 4.8M4.8 11.2L3.8 12.2" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
                  </svg>
                  <span className="nav-text">{t("studio.settingsBtn")}</span>
                </span>
              </button>
            </div>
          </aside>

          <section className="fluent-list-col">
            <div className="list-col-head">
              <h2>{centerTitle}</h2>
              {view.kind === "trash" ? (
                trashPhrases.length > 0 && (
                  <button
                    type="button"
                    className="btn btn-danger btn-sm"
                    onClick={() => void handleEmptyTrash()}
                  >
                    {t("studio.emptyTrash")}
                  </button>
                )
              ) : view.kind === "clipboard" ? (
                clipboardItems.length > 0 && (
                  <button
                    type="button"
                    className="btn btn-danger btn-sm"
                    onClick={() => void handleClearClipboard()}
                  >
                    {t("studio.clearClipboard")}
                  </button>
                )
              ) : (
                <div className="row-actions" onClick={(e) => e.stopPropagation()}>
                  {view.kind !== "stats" && displayedPhrases.length > 0 && (
                    <div className="dropdown-wrap">
                      <button
                        type="button"
                        className="btn btn-sm"
                        title={t("studio.exportListBtn")}
                        onClick={() => setShowListExportMenu((v) => !v)}
                      >
                        ↓
                      </button>
                      {showListExportMenu && (
                        <div className="dropdown-menu">
                          <button
                            type="button"
                            onClick={() =>
                              void handleExportSubset(
                                displayedPhrases.map((p) => p.id),
                                "meetin",
                                centerTitle.toLocaleLowerCase("tr-TR").replace(/\s+/g, "-"),
                              )
                            }
                          >
                            {t("studio.exportMeetin")}
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              void handleExportSubset(
                                displayedPhrases.map((p) => p.id),
                                "csv",
                                centerTitle.toLocaleLowerCase("tr-TR").replace(/\s+/g, "-"),
                              )
                            }
                          >
                            {t("studio.exportCsv")}
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              void handleExportSubset(
                                displayedPhrases.map((p) => p.id),
                                "ahk",
                                centerTitle.toLocaleLowerCase("tr-TR").replace(/\s+/g, "-"),
                              )
                            }
                          >
                            {t("studio.exportAhk")}
                          </button>
                        </div>
                      )}
                    </div>
                  )}
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    title={t("tip.newPhrase")}
                    onClick={() => openNewDraft()}
                  >
                    + {t("toolbar.newPhrase")}
                  </button>
                </div>
              )}
            </div>

            {selection.ids.length > 1 &&
              view.kind !== "clipboard" &&
              view.kind !== "stats" &&
              view.kind !== "trash" && (
                <div
                  className="bulk-selection-bar"
                  role="status"
                  title={t("studio.bulkHint")}
                  onClick={(e) => e.stopPropagation()}
                >
                  <span className="bulk-count">
                    {t("studio.bulkSelected", { count: selection.ids.length })}
                  </span>
                  <button
                    type="button"
                    className="mini-icon-btn"
                    title={t("studio.bulkClear")}
                    aria-label={t("studio.bulkClear")}
                    onClick={() => setSelection((prev) => ({ ...prev, ids: [] }))}
                  >
                    ×
                  </button>
                </div>
              )}

            {view.kind !== "stats" && (
              <div className="list-filter-bar">
                <input
                  className="filter-input"
                  type="search"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder={t("toolbar.searchPlaceholder")}
                />
                {(view.kind === "all" ||
                  view.kind === "folder" ||
                  view.kind === "recent") && (
                  <select
                    className="sort-pill-select"
                    value={sortMode}
                    onChange={(e) => setSortMode(e.target.value as SortMode)}
                    aria-label={t("studio.sortLabel")}
                  >
                    <option value="updated">{t("studio.sortUpdated")}</option>
                    <option value="uses">{t("studio.sortUses")}</option>
                    <option value="abbr">{t("studio.sortAbbr")}</option>
                    <option value="recent">{t("studio.sortRecent")}</option>
                  </select>
                )}
              </div>
            )}

            <div className="list-scroll">
              {view.kind === "clipboard" ? (
                filteredClipboard.length === 0 ? (
                  <div className="empty-state">
                    <p className="muted small">{t("studio.clipboardEmpty")}</p>
                  </div>
                ) : (
                  filteredClipboard.map((item) => (
                    <div key={item.id} className="phrase-list-row">
                      <div className="row-top-line">
                        <span className="muted small">
                          {formatRelativeDate(item.copied_at)}
                        </span>
                        {item.source_process && (
                          <span className="tag-chip">{item.source_process}</span>
                        )}
                        <button
                          type="button"
                          className="link-btn"
                          style={{ marginLeft: "auto" }}
                          onClick={() =>
                            openNewDraft(item.content, item.content_html)
                          }
                        >
                          {t("studio.convertToPhrase")}
                        </button>
                      </div>
                      <div className="row-preview-line">{item.content}</div>
                      <div className="clip-actions-row">
                        <button
                          type="button"
                          className="btn btn-xs"
                          onClick={() =>
                            void handleClipboardTransformCopy(
                              item.content,
                              "original",
                              t("studio.copyOriginal"),
                            )
                          }
                        >
                          {t("studio.copyOriginal")}
                        </button>
                        <button
                          type="button"
                          className="btn btn-xs"
                          onClick={() =>
                            void handleClipboardTransformCopy(
                              item.content,
                              "upper",
                              t("studio.copyUpper"),
                            )
                          }
                        >
                          {t("studio.copyUpper")}
                        </button>
                        <button
                          type="button"
                          className="btn btn-xs"
                          onClick={() =>
                            void handleClipboardTransformCopy(
                              item.content,
                              "lower",
                              t("studio.copyLower"),
                            )
                          }
                        >
                          {t("studio.copyLower")}
                        </button>
                        <button
                          type="button"
                          className="btn btn-xs"
                          onClick={() =>
                            void handleClipboardTransformCopy(
                              item.content,
                              "title",
                              t("studio.copyTitle"),
                            )
                          }
                        >
                          {t("studio.copyTitle")}
                        </button>
                      </div>
                    </div>
                  ))
                )
              ) : view.kind === "stats" ? (
                <div className="stats-compact-list">
                  <div className="stats-mini-grid">
                    <div className="usage-stat-card">
                      <strong>{stats?.today_count ?? 0}</strong>
                      <span className="muted small">{t("studio.statsToday")}</span>
                    </div>
                    <div className="usage-stat-card">
                      <strong>{stats?.total_count ?? 0}</strong>
                      <span className="muted small">{t("studio.statsTotal")}</span>
                    </div>
                  </div>
                  <div className="sidebar-section-head" style={{ marginTop: 8 }}>
                    <span>{t("studio.statsTopTitle")}</span>
                  </div>
                  {stats?.top_phrases.map((tp) => (
                    <div key={tp.id} className="phrase-list-row">
                      <div className="row-top-line">
                        <span className="abbr-chip">{tp.abbr}</span>
                        <span className="row-date">
                          {tp.use_count} × · {formatRelativeDate(tp.last_used_at)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              ) : view.kind === "trash" ? (
                displayedPhrases.length === 0 ? (
                  <div className="empty-state">
                    <p className="muted small">{t("studio.trashEmpty")}</p>
                  </div>
                ) : (
                  displayedPhrases.map((p) => (
                    <div key={p.id} className="phrase-list-row">
                      <div className="row-top-line">
                        <span className="abbr-chip">{p.abbr}</span>
                        <strong className="row-title">
                          {p.description || formatPreviewLine(p.content)}
                        </strong>
                      </div>
                      <div className="row-preview-line">
                        {formatPreviewLine(p.content)}
                      </div>
                      <div className="row-actions" style={{ marginTop: 6 }}>
                        <button
                          type="button"
                          className="btn btn-sm"
                          onClick={() => void handleRestorePhrase(p.id)}
                        >
                          {t("studio.restore")}
                        </button>
                        <button
                          type="button"
                          className="btn btn-danger btn-sm"
                          onClick={() => void handleHardDelete(p.id)}
                        >
                          {t("studio.hardDelete")}
                        </button>
                      </div>
                    </div>
                  ))
                )
              ) : displayedPhrases.length === 0 ? (
                <div className="empty-state">
                  <p className="muted">{t("list.emptyTitle")}</p>
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    onClick={() => openNewDraft()}
                  >
                    + {t("toolbar.newPhrase")}
                  </button>
                </div>
              ) : (
                displayedPhrases.map((p) => {
                  const isOpen = draft?.id === p.id;
                  // Çoklu seçimde yalnız seçilenler vurgulanır; açık satır sol çizgisini korur
                  const isSelected =
                    selection.ids.length > 0 ? selection.ids.includes(p.id) : isOpen;
                  const isSwiped = swipedId === p.id;
                  const title =
                    p.description.trim() || formatPreviewLine(p.content);
                  return (
                    <div
                      key={p.id}
                      className={`swipe-row-wrap ${isSwiped ? "is-swiped" : ""}`}
                      onWheel={(e) => {
                        if (Math.abs(e.deltaX) > Math.abs(e.deltaY) && Math.abs(e.deltaX) > 16) {
                          setSwipedId(e.deltaX > 0 ? p.id : null);
                        }
                      }}
                    >
                      <button
                        type="button"
                        className="swipe-delete-btn"
                        tabIndex={isSwiped ? 0 : -1}
                        onClick={(e) => {
                          e.stopPropagation();
                          void handleSoftDelete(p.id);
                          setSwipedId(null);
                        }}
                      >
                        {t("studio.swipeDelete")}
                      </button>
                      <div
                        draggable
                        onDragStart={(e) => {
                          const dragIds = actionIds(selection, displayedOrder, p.id);
                          e.dataTransfer.setData("text/plain", String(p.id));
                          e.dataTransfer.setData(
                            "application/x-meetin-phrase-ids",
                            JSON.stringify(dragIds),
                          );
                          e.dataTransfer.effectAllowed = "copyMove";
                        }}
                        onContextMenu={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          setContextMenu({
                            kind: "phrase",
                            x: Math.min(e.clientX, window.innerWidth - 230),
                            y: Math.min(e.clientY, window.innerHeight - 280),
                            phrase: p,
                            ids: actionIds(selection, displayedOrder, p.id),
                          });
                        }}
                        onMouseDown={(e) => {
                          // Shift+tık metin seçmesin
                          if (e.shiftKey) e.preventDefault();
                        }}
                        onClick={(e) => handleRowClick(p, e)}
                        title={t("studio.rowHint")}
                        className={`phrase-list-row is-clickable swipe-row-front ${isSelected ? "is-selected" : ""} ${isOpen ? "is-open" : ""}`}
                        aria-current={isOpen ? "true" : undefined}
                      >
                        <div className="row-top-line">
                          <span className="abbr-chip">{p.abbr}</span>
                          <strong className="row-title">{title}</strong>
                          <span className="row-date">
                            {formatRelativeDate(p.last_used_at ?? p.updated_at)}
                          </span>
                        </div>
                        <div className="row-preview-line">
                          {formatPreviewLine(p.content)}
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </section>

          <main className="fluent-detail-col">
            {!draft ? (
              <div className="empty-state" style={{ margin: "auto" }}>
                <p className="muted">{t("studio.selectOrCreate")}</p>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => openNewDraft()}
                >
                  + {t("toolbar.newPhrase")}
                </button>
              </div>
            ) : (
              <form onSubmit={handleSubmit} className="detail-canvas">
                <div className="detail-header-row">
                  <div className="detail-title-wrap">
                    <span className="tiny-label">{t("studio.descriptionTitle")}</span>
                    <input
                      className="detail-title-input"
                      value={draft.description}
                      onChange={(e) => {
                        setDescriptionManual(true);
                        setDraft({ ...draft, description: e.target.value });
                      }}
                      placeholder={t("form.descriptionPlaceholder")}
                    />
                  </div>
                  <div className="row-actions" style={{ alignItems: "center" }}>
                    {autoSaveState !== "idle" && (
                      <span className="autosave-status">
                        {autoSaveState === "saving"
                          ? t("studio.autoSaving")
                          : t("studio.autoSaved")}
                      </span>
                    )}
                    {draft.id !== null && (
                      <button
                        type="button"
                        title={t("tip.historyToggle")}
                        className={`btn btn-sm ${showHistoryDrawer ? "is-active-outline" : ""}`}
                        onClick={() => setShowHistoryDrawer((v) => !v)}
                      >
                        {t("studio.historyPill", { count: versions.length })}
                      </button>
                    )}
                  </div>
                </div>

                <TagInput
                  value={draft.tags}
                  onChange={(tags) => setDraft((d) => (d ? { ...d, tags } : d))}
                  placeholder={t("studio.addTagPlaceholder")}
                  inputLabel={t("studio.tagInputLabel")}
                  removeLabel={(tag) => t("studio.removeTag", { tag })}
                />

                <StackPanel
                  open={showHistoryDrawer}
                  items={versions.map((ver) => ({
                    id: ver.id,
                    meta: formatVersionTime(ver.created_at),
                    preview: formatPreviewLine(ver.content),
                  }))}
                  title={t("studio.tabHistory")}
                  closeLabel={t("studio.closeHistory")}
                  emptyText={t("studio.historyEmpty")}
                  actionLabel={t("studio.restoreVersion")}
                  onAction={(id) => void handleRestoreVersion(id)}
                  onClose={() => setShowHistoryDrawer(false)}
                />

                <div className="content-card-section">
                  <span className="tiny-label">{t("studio.tabContent")}</span>
                  <div className="unified-content-card" onClick={(e) => e.stopPropagation()}>
                    <RichEditor
                      draftKey={`${editorSessionId}-${draft.id ?? "new"}`}
                      content={draft.content}
                      contentHtml={draft.content_html}
                      placeholder={t("form.contentPlaceholder")}
                      variableSnippets={VARIABLE_SNIPPETS}
                      onRequestInsertImage={() => imageInputRef.current?.click()}
                      onChange={(nextContent, nextHtml) => {
                        setDraft((prev) =>
                          prev
                            ? {
                                ...prev,
                                content: nextContent,
                                content_html: nextHtml,
                                description: descriptionManual
                                  ? prev.description
                                  : deriveAutoDescription(nextContent),
                              }
                            : prev,
                        );
                      }}
                    />
                    <input
                      ref={imageInputRef}
                      type="file"
                      accept="image/png,image/jpeg,image/gif,image/webp,image/svg+xml"
                      style={{ display: "none" }}
                      onChange={handleInsertImageFile}
                    />

                    {contentSegments.some((s) => s.isVar) && (
                      <div className="var-preview-strip">
                        {contentSegments.map((seg, i) =>
                          seg.isVar ? (
                            <span key={i} className="inline-var-pill">
                              {seg.value}
                            </span>
                          ) : (
                            <span key={i}>{seg.value}</span>
                          ),
                        )}
                      </div>
                    )}

                    {embeddedImages.length > 0 && (
                      <div className="rich-image-preview-strip">
                        <span className="muted small">
                          {t("studio.embeddedImages", {
                            count: embeddedImages.length,
                          })}
                        </span>
                        <div className="rich-image-grid">
                          {embeddedImages.map((img) => (
                            <div key={img.key} className="rich-image-card">
                              <img
                                src={img.src}
                                alt={img.alt || t("studio.imageAltFallback")}
                                className="rich-image-thumb"
                              />
                              <button
                                type="button"
                                className="rich-image-remove"
                                title={t("studio.removeImage")}
                                aria-label={t("studio.removeImage")}
                                onClick={() =>
                                  setDraft((prev) =>
                                    prev && prev.content_html
                                      ? {
                                          ...prev,
                                          content_html: removeEmbeddedImageAt(
                                            prev.content_html,
                                            img.matchStart,
                                            img.matchEnd,
                                          ),
                                        }
                                      : prev,
                                  )
                                }
                              >
                                ×
                              </button>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <div className="meta-two-col">
                  <div className="meta-field-group">
                    <span className="tiny-label">{t("form.abbrLabel")}</span>
                    <div className="unified-abbr-box" onClick={(e) => e.stopPropagation()}>
                      <input
                        className="abbr-input-bold"
                        value={draft.abbr}
                        onChange={(e) =>
                          setDraft({ ...draft, abbr: e.target.value })
                        }
                        placeholder={t("form.abbrPlaceholder")}
                      />
                      <div className="dropdown-wrap">
                        <button
                          type="button"
                          className="case-mode-symbol-btn"
                          title={`${t("form.caseModeLabel")}: ${t(CASE_MODE_SYMBOLS[draft.case_mode].labelKey)} — ${t(CASE_MODE_SYMBOLS[draft.case_mode].descKey)}`}
                          onClick={() => setShowCaseMenu((v) => !v)}
                        >
                          {CASE_MODE_SYMBOLS[draft.case_mode].symbol}
                        </button>
                        {showCaseMenu && (
                          <div className="dropdown-menu case-mode-popover">
                            <div className="case-popover-head">
                              {t("studio.caseModeMenuTitle")}
                            </div>
                            {(["match_typed", "keep_original", "exact_match"] as const).map(
                              (mode) => (
                                <button
                                  key={mode}
                                  type="button"
                                  className={`case-popover-item ${draft.case_mode === mode ? "is-selected" : ""}`}
                                  onClick={() => {
                                    setDraft({ ...draft, case_mode: mode });
                                    setShowCaseMenu(false);
                                  }}
                                >
                                  <span className="abbr-chip">
                                    {CASE_MODE_SYMBOLS[mode].symbol}
                                  </span>
                                  <div className="case-popover-meta">
                                    <strong>{t(CASE_MODE_SYMBOLS[mode].labelKey)}</strong>
                                    <span className="muted small">
                                      {t(CASE_MODE_SYMBOLS[mode].descKey)}
                                    </span>
                                  </div>
                                </button>
                              ),
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>

                  <div className="meta-field-group">
                    <span className="tiny-label">{t("studio.shortcutLabel")}</span>
                    <div className="unified-abbr-box">
                      <ShortcutField
                        value={draft.shortcut}
                        label={t("studio.shortcutLabel")}
                        title={t("settings.shortcutFieldTip")}
                        onKeyDown={(e) => {
                          if (e.key === "Backspace" || e.key === "Delete") {
                            e.preventDefault();
                            setDraft({ ...draft, shortcut: "" });
                            return;
                          }
                          const combo = captureShortcutCombo(e);
                          if (combo) {
                            e.preventDefault();
                            setDraft({ ...draft, shortcut: combo });
                          }
                        }}
                        placeholder="Ctrl+Alt+R"
                      />
                      <select
                        className="inline-case-select"
                        value=""
                        onChange={(e) => {
                          const v = e.target.value;
                          if (!v) {
                            setDraft({ ...draft, folder_id: null, folder_ids: [] });
                            return;
                          }
                          const fid = Number(v);
                          if (!draft.folder_ids.includes(fid)) {
                            const nextIds = [...draft.folder_ids, fid];
                            const fname = folders.find((f) => f.id === fid)?.name;
                            const nextTags =
                              fname && !draft.tags.includes(fname)
                                ? [...draft.tags, fname]
                                : draft.tags;
                            setDraft({
                              ...draft,
                              folder_id: nextIds[0] ?? null,
                              folder_ids: nextIds,
                              tags: nextTags,
                            });
                          }
                        }}
                        title={t("studio.folderLabel")}
                      >
                        <option value="">
                          {draft.folder_ids.length === 0
                            ? t("studio.unfiled")
                            : `+ ${t("studio.folderLabel")}`}
                        </option>
                        {folders.map((f) => (
                          <option key={f.id} value={f.id}>
                            {f.name}
                          </option>
                        ))}
                      </select>
                    </div>
                    {draft.folder_ids.length > 0 && (
                      <div className="folder-chips-row">
                        {draft.folder_ids.map((fid) => {
                          const fObj = folders.find((f) => f.id === fid);
                          const fname = fObj?.name ?? `#${fid}`;
                          return (
                            <span key={fid} className="tag-pill">
                              {fObj?.color && (
                                <span className={`folder-color-dot folder-dot-${fObj.color}`} />
                              )}
                              {fname}
                              <button
                                type="button"
                                onClick={() => {
                                  const nextIds = draft.folder_ids.filter(
                                    (id) => id !== fid,
                                  );
                                  setDraft({
                                    ...draft,
                                    folder_id: nextIds[0] ?? null,
                                    folder_ids: nextIds,
                                  });
                                }}
                              >
                                ×
                              </button>
                            </span>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>

                <div className="rules-inline-section">
                  <span className="tiny-label">{t("studio.tabRules")}</span>
                  <div className="rules-chip-row">
                    <span className="muted small">
                      {appRules.length === 0
                        ? t("studio.rulesAllApps")
                        : t("studio.rulesScopedLabel")}
                    </span>
                    {appRules.map((r, idx) => (
                      <span key={idx} className="tag-pill">
                        {r.kind === "exclude" ? "≠ " : ""}
                        {r.process_name}
                        <button
                          type="button"
                          onClick={() =>
                            void persistRules(
                              appRules.filter((_, i) => i !== idx),
                            )
                          }
                        >
                          ×
                        </button>
                      </span>
                    ))}
                    <button
                      type="button"
                      className="link-btn"
                      title={t("tip.addRule")}
                      onClick={() => {
                        const next = !showRuleAdd;
                        setShowRuleAdd(next);
                        if (next) {
                          void invoke<RunningAppInfo[]>("list_running_apps")
                            .then(setRunningApps)
                            .catch(() => {});
                        }
                      }}
                    >
                      {t("studio.addRule")}
                    </button>
                  </div>

                  {showRuleAdd && (
                    <div className="inline-rule-panel">
                      <div className="inline-rule-editor">
                        <select
                          className="select"
                          style={{ width: 180 }}
                          value={newRuleKind}
                          onChange={(e) =>
                            setNewRuleKind(e.target.value as AppRule["kind"])
                          }
                        >
                          <option value="only">{t("studio.ruleOnly")}</option>
                          <option value="exclude">{t("studio.ruleExclude")}</option>
                        </select>
                        <input
                          className="input"
                          value={newRuleProc}
                          onChange={(e) => setNewRuleProc(e.target.value)}
                          placeholder={t("studio.customProcess")}
                        />
                        <button
                          type="button"
                          className="btn btn-sm"
                          onClick={() => {
                            const proc = newRuleProc.trim();
                            if (!proc) return;
                            void persistRules([
                              ...appRules,
                              {
                                kind: newRuleKind,
                                process_name: proc,
                                title_contains: null,
                              },
                            ]);
                            setNewRuleProc("");
                            setShowRuleAdd(false);
                          }}
                        >
                          {t("form.save")}
                        </button>
                      </div>
                      <div className="running-apps-picker">
                        <div className="running-apps-head">
                          <span className="muted small">{t("studio.detectApps")}</span>
                          <button
                            type="button"
                            className="link-btn"
                            onClick={() =>
                              void invoke<RunningAppInfo[]>("list_running_apps")
                                .then(setRunningApps)
                                .catch(() => {})
                            }
                          >
                            {t("studio.refreshApps")}
                          </button>
                        </div>
                        {runningApps.length === 0 ? (
                          <span className="muted small">{t("studio.detectAppsEmpty")}</span>
                        ) : (
                          <div className="running-apps-chips">
                            {runningApps.map((app) => (
                              <button
                                key={app.process_name}
                                type="button"
                                className="btn btn-xs"
                                title={app.window_title}
                                onClick={() => {
                                  const already = appRules.some(
                                    (r) =>
                                      r.kind === newRuleKind &&
                                      r.process_name.toLowerCase() ===
                                        app.process_name.toLowerCase(),
                                  );
                                  if (!already) {
                                    void persistRules([
                                      ...appRules,
                                      {
                                        kind: newRuleKind,
                                        process_name: app.process_name,
                                        title_contains: null,
                                      },
                                    ]);
                                  }
                                  setShowRuleAdd(false);
                                }}
                              >
                                + {app.process_name}
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>

                {selectedPhrase && (
                  <div className="usage-cards-row">
                    <div className="usage-stat-card">
                      <strong>{selectedPhrase.use_count}</strong>
                      <span className="muted small">{t("studio.usedTimes")}</span>
                    </div>
                    <div className="usage-stat-card">
                      <strong>
                        {formatRelativeDate(selectedPhrase.last_used_at)}
                      </strong>
                      <span className="muted small">{t("studio.lastUsedLabel")}</span>
                    </div>
                  </div>
                )}

                <div className="detail-footer-actions">
                  {draft.id !== null && (
                    <label className="checkbox-inline" style={{ marginRight: "auto" }}>
                      <input
                        type="checkbox"
                        checked={draft.enabled}
                        onChange={(e) =>
                          setDraft({ ...draft, enabled: e.target.checked })
                        }
                      />
                      <span>{t("studio.enabledLabel")}</span>
                    </label>
                  )}
                  {draft.id !== null && (
                    <button
                      type="button"
                      className="btn btn-danger btn-sm"
                      onClick={() => void handleSoftDelete(draft.id!)}
                    >
                      {t("list.delete")}
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={() => setDraft(null)}
                  >
                    {t("form.cancel")}
                  </button>
                  <button type="submit" className="btn btn-primary btn-sm">
                    {t("form.save")}
                  </button>
                </div>
              </form>
            )}
          </main>
        </div>
      )}

      {showCsv && (
        <CsvImportModal
          onClose={() => setShowCsv(false)}
          onImported={() => void loadAll()}
        />
      )}

      {(showBulkModal || importPreview) && (
        <BulkAddModal
          folders={folders}
          initialItems={importPreview?.items}
          sourceTitle={importPreview?.filename}
          onClose={() => {
            setShowBulkModal(false);
            setImportPreview(null);
          }}
          onDone={handleImportDone}
        />
      )}

      {showPalette && (
        <CommandPalette
          phrases={phrases}
          commands={paletteCommands}
          onSelectPhrase={(p) => {
            setPageMode("studio");
            setView({ kind: "all" });
            startEdit(p);
          }}
          onClose={() => setShowPalette(false)}
        />
      )}

      {showOnboarding && (
        <OnboardingModal
          initialName={userName}
          onFinish={() => {
            setShowOnboarding(false);
            void loadAll();
          }}
        />
      )}

      {contextMenu && (
        <div
          className="dropdown-menu floating-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          {contextMenu.kind === "phrase" ? (
            <>
              {contextMenu.ids.length > 1 ? (
                <div className="case-popover-head">
                  {t("studio.bulkSelected", { count: contextMenu.ids.length })}
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => {
                    startEdit(contextMenu.phrase);
                    setContextMenu(null);
                  }}
                >
                  {t("studio.ctxEdit")}
                </button>
              )}
              {view.kind === "folder" && (
                <button
                  type="button"
                  onClick={() => {
                    const ids = contextMenu.ids;
                    setContextMenu(null);
                    void handleRemoveFromFolder(ids, view.folderId);
                  }}
                >
                  {t("studio.ctxRemoveFromFolder")}
                </button>
              )}
              {folders.length > 0 && (
                <>
                  <div className="menu-divider" />
                  <div className="case-popover-head">{t("studio.ctxMoveTo")}</div>
                  {folders.map((f) => (
                    <button
                      key={`mv-${f.id}`}
                      type="button"
                      onClick={() => {
                        const ids = contextMenu.ids;
                        setContextMenu(null);
                        void handleFolderDrop(f.id, ids, false);
                      }}
                    >
                      → {f.name}
                    </button>
                  ))}
                  <div className="case-popover-head">{t("studio.ctxCopyTo")}</div>
                  {folders.map((f) => (
                    <button
                      key={`cp-${f.id}`}
                      type="button"
                      onClick={() => {
                        const ids = contextMenu.ids;
                        setContextMenu(null);
                        void handleFolderDrop(f.id, ids, true);
                      }}
                    >
                      + {f.name}
                    </button>
                  ))}
                </>
              )}
              <div className="menu-divider" />
              <button
                type="button"
                onClick={() =>
                  void handleExportSubset(
                    contextMenu.ids,
                    "meetin",
                    contextMenu.ids.length > 1 ? "meetin-secilenler" : contextMenu.phrase.abbr,
                  )
                }
              >
                {t("studio.ctxExportMee")}
              </button>
              <button
                type="button"
                onClick={() =>
                  void handleExportSubset(
                    contextMenu.ids,
                    "csv",
                    contextMenu.ids.length > 1 ? "meetin-secilenler" : contextMenu.phrase.abbr,
                  )
                }
              >
                {t("studio.ctxExportCsv")}
              </button>
              <div className="menu-divider" />
              <button
                type="button"
                className="btn-danger"
                onClick={() => {
                  const ids = contextMenu.ids;
                  setContextMenu(null);
                  void handleBulkDelete(ids);
                }}
              >
                {t("studio.ctxDelete")}
              </button>
            </>
          ) : (
            <>
              <div className="case-popover-head">{t("studio.ctxFolderColor")}</div>
              <div className="folder-color-palette">
                {FOLDER_COLORS.map((c) => (
                  <button
                    key={c}
                    type="button"
                    className={`folder-color-swatch folder-dot-${c} ${contextMenu.folder.color === c ? "is-active" : ""}`}
                    onClick={() => {
                      const fid = contextMenu.folder.id;
                      setContextMenu(null);
                      void handleUpdateFolder(fid, null, c);
                    }}
                  />
                ))}
                <button
                  type="button"
                  className="btn btn-xs"
                  onClick={() => {
                    const fid = contextMenu.folder.id;
                    setContextMenu(null);
                    void handleUpdateFolder(fid, null, "");
                  }}
                >
                  {t("studio.ctxFolderNoColor")}
                </button>
              </div>
              <div className="menu-divider" />
              <button
                type="button"
                onClick={() => {
                  setRenamingFolderId(contextMenu.folder.id);
                  setRenameFolderDraft(contextMenu.folder.name);
                  setContextMenu(null);
                }}
              >
                {t("studio.ctxRenameFolder")}
              </button>
              <button
                type="button"
                onClick={() => {
                  const f = contextMenu.folder;
                  const fIds = activePhrases
                    .filter(
                      (p) => p.folder_ids?.includes(f.id) || p.folder_id === f.id,
                    )
                    .map((p) => p.id);
                  void handleExportSubset(fIds, "meetin", f.name);
                }}
              >
                {t("studio.ctxExportFolder")}
              </button>
              <button
                type="button"
                onClick={() => {
                  const f = contextMenu.folder;
                  const fIds = activePhrases
                    .filter(
                      (p) => p.folder_ids?.includes(f.id) || p.folder_id === f.id,
                    )
                    .map((p) => p.id);
                  void handleExportSubset(fIds, "csv", f.name);
                }}
              >
                {t("studio.ctxExportFolderCsv")}
              </button>
              <div className="menu-divider" />
              <button
                type="button"
                className="btn-danger"
                onClick={() => {
                  const fid = contextMenu.folder.id;
                  setContextMenu(null);
                  void handleDeleteFolder(fid);
                }}
              >
                {t("studio.deleteFolder")}
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
