import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type FormEvent,
  type SetStateAction,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { suggestAbbreviation } from "../island/abbrSuggest";
import { EMPTY_SELECTION, type ListSelection } from "./listSelection";
import { uniqueCopyAbbr } from "./phraseCopy";
import {
  deriveAutoDescription,
  EMPTY_DRAFT,
  type EditorDraft,
  type PageMode,
  type SidebarView,
} from "./studioHelpers";
import type {
  AbbrSuggestMode,
  AppRule,
  ClipboardHistoryRecord,
  EngineStatus,
  FolderRecord,
  PhraseRecord,
  PhraseVersionRecord,
  RunningAppInfo,
  UsageStats,
} from "./types";

interface StudioEditorOptions {
  initialSelectedPhraseId: number | null | undefined;
  view: SidebarView;
  pageMode: PageMode;
  setPageMode: Dispatch<SetStateAction<PageMode>>;
  setSelection: Dispatch<SetStateAction<ListSelection>>;
  setError: Dispatch<SetStateAction<string | null>>;
  setNotice: Dispatch<SetStateAction<string | null>>;
}

/**
 * Kitaplık verisi ve düzenleyici döngüsü tek yerde (R4.2): yükleme, düzenlemeye başlama,
 * yeni taslak, kaydetme ve 650 ms otomatik kaydetme birbirine bağlı olduğu için ayrılmaz.
 */
export function useStudioEditor({
  initialSelectedPhraseId,
  view,
  pageMode,
  setPageMode,
  setSelection,
  setError,
  setNotice,
}: StudioEditorOptions) {
  const [phrases, setPhrases] = useState<PhraseRecord[]>([]);
  const [folders, setFolders] = useState<FolderRecord[]>([]);
  const [clipboardItems, setClipboardItems] = useState<ClipboardHistoryRecord[]>([]);
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [abbrSuggestMode, setAbbrSuggestMode] = useState<AbbrSuggestMode>("initials");

  /** "Yeni ifade" ile açılan form: Kaydet sonrası boşalır, sıradaki ifade yazılır (R3.1a) */
  const newPhraseSessionRef = useRef(false);

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
  }, [setError]);

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
      newPhraseSessionRef.current = false;
      setEditorSessionId((n) => n + 1);
      setDraft(nextDraft);
      void loadPhraseMeta(p.id);
    },
    [draftSnapshot, loadPhraseMeta, setError, setNotice],
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
        const prefId = initialSelectedPhraseId;
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
  }, [initialSelectedPhraseId, setError, startEdit]);

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
      newPhraseSessionRef.current = true;
      setEditorSessionId((n) => n + 1);
      setDraft(next);
    },
    [abbrSuggestMode, folders, view, setPageMode, setError, setNotice],
  );

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
        } else if (newPhraseSessionRef.current) {
          // Yeni ifade kaydedildi: form boşalır, kaydedilen listede seçili kalır (R3.1a)
          openNewDraft();
          setSelection({ ids: [saved.id], anchor: saved.id });
          setNotice(t("studio.savedNotice", { abbr: saved.abbr }));
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
    [appRules, draftSnapshot, loadAll, openNewDraft, startEdit, setError, setNotice, setSelection],
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

  async function refreshRunningApps() {
    try {
      setRunningApps(await invoke<RunningAppInfo[]>("list_running_apps"));
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  /** Açık ifadenin kopyası: içerik, etiketler, klasörler ve kurallar; kısaltma çakışmasız (R3.1a) */
  async function handleDuplicate() {
    if (!draft) return;
    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
      autoSaveTimerRef.current = null;
    }
    // Asıl ifadede kaydedilmemiş değişiklik varsa önce o kaydedilir
    if (draftSnapshot(draft) !== lastSavedSnapshotRef.current) {
      await persistDraft(draft, true);
    }
    const existing = phrases.filter((p) => p.deleted_at === null).map((p) => p.abbr);
    const copy: EditorDraft = {
      ...draft,
      id: null,
      abbr: uniqueCopyAbbr(draft.abbr, existing, status?.fold_dotted_i ?? false),
      description: draft.description.trim()
        ? t("studio.copyDescription", { description: draft.description.trim() })
        : "",
      folder_ids: [...draft.folder_ids],
      tags: [...draft.tags],
      // Kısayol tuşu tektir; kopya aynı tuşa bağlanmaz
      shortcut: "",
    };
    setError(null);
    setShowHistoryDrawer(false);
    setVersions([]);
    setAutoSaveState("idle");
    setDescriptionManual(copy.description.length > 0);
    lastSavedSnapshotRef.current = "";
    newPhraseSessionRef.current = false;
    setSelection(EMPTY_SELECTION);
    setEditorSessionId((n) => n + 1);
    setDraft(copy);
    setNotice(t("studio.duplicatedNotice", { abbr: draft.abbr }));
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

  return {
    phrases,
    folders,
    clipboardItems,
    stats,
    status,
    setAbbrSuggestMode,
    draft,
    setDraft,
    descriptionManual,
    setDescriptionManual,
    autoSaveState,
    appRules,
    setAppRules,
    runningApps,
    newRuleProc,
    setNewRuleProc,
    newRuleKind,
    setNewRuleKind,
    showRuleAdd,
    setShowRuleAdd,
    showCaseMenu,
    setShowCaseMenu,
    versions,
    setVersions,
    showHistoryDrawer,
    setShowHistoryDrawer,
    editorSessionId,
    setEditorSessionId,
    autoSelectedRef,
    startEdit,
    loadAll,
    openNewDraft,
    handleSubmit,
    refreshRunningApps,
    handleDuplicate,
    handleRestoreVersion,
  };
}

export type StudioEditorState = ReturnType<typeof useStudioEditor>;
