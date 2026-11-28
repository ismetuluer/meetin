import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t, useLanguage } from "../i18n";
import { BulkAddModal } from "./BulkAddModal";
import { CommandPalette, type PaletteCommand } from "./CommandPalette";
import { CsvImportModal } from "./CsvImportModal";
import { HelpView } from "./HelpView";
import { OnboardingModal } from "./OnboardingModal";
import { SettingsView } from "./SettingsView";
import { EMPTY_SELECTION, pruneSelection, type ListSelection } from "./listSelection";
import { ExportDialog, type ExportFormat, type ExportRequest } from "./ExportDialog";
import { StudioBanners } from "./StudioBanners";
import { StudioContextMenu } from "./StudioContextMenu";
import { StudioEditor } from "./StudioEditor";
import { StudioPhraseList } from "./StudioPhraseList";
import { StudioSidebar } from "./StudioSidebar";
import { StudioTopbar } from "./StudioTopbar";
import { createExportFile, downloadBlob } from "./studioExport";
import {
  EMPTY_DRAFT,
  filterClipboardItems,
  filterDisplayedPhrases,
  loadUiPrefs,
  NOTICE_MS,
  saveUiPrefs,
  viewTitle,
  type ContextMenuState,
  type PageMode,
  type SidebarView,
  type SortMode,
} from "./studioHelpers";
import { useStudioEditor } from "./useStudioEditor";
import { useStudioShortcuts } from "./useStudioShortcuts";
import type {
  AppSettingsDto,
  CsvImportSummary,
  FolderRecord,
  ImportedPhraseItem,
  PhraseRecord,
} from "./types";
import "./studio.css";

/**
 * Uzman arayüzü kabuğu (R4.2): sayfa durumu ve klasör/liste işlemleri burada;
 * kitaplık verisi ile düzenleyici döngüsü `useStudioEditor`, görünüm parçaları `Studio*.tsx`.
 */
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
  const [exportRequest, setExportRequest] = useState<ExportRequest | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
  const [renamingFolderId, setRenamingFolderId] = useState<number | null>(null);
  const [renameFolderDraft, setRenameFolderDraft] = useState("");
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

  const editor = useStudioEditor({
    initialSelectedPhraseId: initialPrefs.selectedPhraseId,
    view,
    pageMode,
    setPageMode,
    setSelection,
    setError,
    setNotice,
  });
  const {
    phrases,
    folders,
    clipboardItems,
    stats,
    status,
    setAbbrSuggestMode,
    draft,
    setDraft,
    setAppRules,
    setVersions,
    setEditorSessionId,
    setShowCaseMenu,
    autoSelectedRef,
    startEdit,
    loadAll,
    openNewDraft,
  } = editor;

  useEffect(() => {
    saveUiPrefs({
      sidebarCollapsed,
      view,
      sortMode,
      selectedPhraseId: draft?.id ?? null,
    });
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
  }, [
    loadAll,
    autoSelectedRef,
    setAbbrSuggestMode,
    setAppRules,
    setDraft,
    setEditorSessionId,
    setVersions,
  ]);

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

  const displayedPhrases = useMemo(
    () => filterDisplayedPhrases(activePhrases, trashPhrases, view, query, sortMode),
    [activePhrases, trashPhrases, view, query, sortMode],
  );

  const displayedOrder = useMemo(() => displayedPhrases.map((p) => p.id), [displayedPhrases]);

  // Filtre, görünüm ya da silme sonrası görünmeyen satırlar seçimden düşer
  useEffect(() => {
    setSelection((prev) => pruneSelection(prev, displayedOrder));
  }, [displayedOrder]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const listSelectable =
    view.kind !== "clipboard" && view.kind !== "stats" && view.kind !== "trash";

  useStudioShortcuts({
    pageMode,
    showPalette,
    showOnboarding,
    listSelectable,
    displayedOrder,
    selection,
    setShowPalette,
    setPageMode,
    setSelection,
  });

  const filteredClipboard = useMemo(
    () => filterClipboardItems(clipboardItems, query),
    [clipboardItems, query],
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

  /** Seçilen biçimde dosyayı oluşturup indirir (R3.4, tek dışa aktarma akışı). */
  async function runExport(request: ExportRequest, format: ExportFormat) {
    setError(null);
    try {
      const { blob, fileName } = await createExportFile(request, format);
      downloadBlob(blob, fileName);
      setExportRequest(null);
      setNotice(t("export.done", { file: fileName }));
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  function openExport(request: ExportRequest) {
    setShowMoreMenu(false);
    setContextMenu(null);
    setExportRequest(request);
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

  async function handleNewFromClipboard() {
    setError(null);
    try {
      const clip = await invoke<string>("read_clipboard_text");
      openNewDraft(clip);
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  function openSettings() {
    setSettingsInitialSection("appearance");
    setPageMode("settings");
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

  const centerTitle = useMemo(() => viewTitle(view, folders), [lang, view, folders]);

  return (
    <div
      className="fluent-shell"
      onClick={() => {
        setShowMoreMenu(false);
        setShowCaseMenu(false);
        setContextMenu(null);
        setSwipedId(null);
      }}
    >
      <StudioTopbar
        pageMode={pageMode}
        status={status}
        showMoreMenu={showMoreMenu}
        setShowMoreMenu={setShowMoreMenu}
        setPageMode={setPageMode}
        setShowPalette={setShowPalette}
        setShowBulkModal={setShowBulkModal}
        setShowCsv={setShowCsv}
        setImportPreview={setImportPreview}
        setError={setError}
        onTogglePause={handleTogglePause}
        onNewFromClipboard={handleNewFromClipboard}
        onOpenLibraryExport={() => openExport({ scope: "library" })}
      />

      <StudioBanners
        error={error}
        notice={notice}
        availableUpdateVersion={availableUpdateVersion}
        showUpdateBanner={pageMode !== "settings"}
        onDismissError={() => setError(null)}
        onOpenUpdate={() => {
          setSettingsInitialSection("update");
          setPageMode("settings");
        }}
        onDismissUpdate={() => setAvailableUpdateVersion(null)}
      />

      {pageMode === "settings" ? (
        <SettingsView
          status={status}
          initialSection={settingsInitialSection}
          onRefresh={() => void loadAll()}
          onOpenExport={() => openExport({ scope: "library" })}
          onReplayOnboarding={() => setShowOnboarding(true)}
        />
      ) : pageMode === "help" ? (
        <HelpView
          onOpenSettings={openSettings}
          onReplayOnboarding={() => setShowOnboarding(true)}
          onCreatePhrase={() => openNewDraft()}
        />
      ) : (
        <div className={`fluent-workspace ${sidebarCollapsed ? "is-sidebar-collapsed" : ""}`}>
          <StudioSidebar
            sidebarCollapsed={sidebarCollapsed}
            setSidebarCollapsed={setSidebarCollapsed}
            view={view}
            setView={setView}
            folders={folders}
            activePhrases={activePhrases}
            trashCount={trashPhrases.length}
            clipboardCount={clipboardItems.length}
            stats={stats}
            showFolderInput={showFolderInput}
            setShowFolderInput={setShowFolderInput}
            newFolderName={newFolderName}
            setNewFolderName={setNewFolderName}
            dropFolderId={dropFolderId}
            setDropFolderId={setDropFolderId}
            dropTrash={dropTrash}
            setDropTrash={setDropTrash}
            renamingFolderId={renamingFolderId}
            setRenamingFolderId={setRenamingFolderId}
            renameFolderDraft={renameFolderDraft}
            setRenameFolderDraft={setRenameFolderDraft}
            setContextMenu={setContextMenu}
            handleCreateFolder={handleCreateFolder}
            handleFolderDrop={handleFolderDrop}
            handleBulkDelete={handleBulkDelete}
            handleUpdateFolder={handleUpdateFolder}
            handleDeleteFolder={handleDeleteFolder}
            onOpenHelp={() => setPageMode("help")}
            onOpenSettings={openSettings}
          />

          <StudioPhraseList
            view={view}
            centerTitle={centerTitle}
            trashCount={trashPhrases.length}
            clipboardCount={clipboardItems.length}
            displayedPhrases={displayedPhrases}
            displayedOrder={displayedOrder}
            filteredClipboard={filteredClipboard}
            stats={stats}
            selection={selection}
            setSelection={setSelection}
            query={query}
            setQuery={setQuery}
            sortMode={sortMode}
            setSortMode={setSortMode}
            swipedId={swipedId}
            setSwipedId={setSwipedId}
            openDraftId={draft?.id ?? null}
            startEdit={startEdit}
            openNewDraft={openNewDraft}
            openExport={openExport}
            setContextMenu={setContextMenu}
            handleSoftDelete={handleSoftDelete}
            loadAll={loadAll}
            setError={setError}
            setNotice={setNotice}
          />

          <StudioEditor
            editor={editor}
            selectedPhrase={selectedPhrase}
            handleSoftDelete={handleSoftDelete}
            setError={setError}
            setNotice={setNotice}
          />
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

      {exportRequest && (
        <ExportDialog
          request={exportRequest}
          libraryCount={activePhrases.length}
          onExport={(format) => runExport(exportRequest, format)}
          onClose={() => setExportRequest(null)}
        />
      )}

      {contextMenu && (
        <StudioContextMenu
          contextMenu={contextMenu}
          setContextMenu={setContextMenu}
          view={view}
          phrases={phrases}
          activePhrases={activePhrases}
          folders={folders}
          startEdit={startEdit}
          openExport={openExport}
          handleFolderDrop={handleFolderDrop}
          handleBulkDelete={handleBulkDelete}
          handleUpdateFolder={handleUpdateFolder}
          handleDeleteFolder={handleDeleteFolder}
          setRenamingFolderId={setRenamingFolderId}
          setRenameFolderDraft={setRenameFolderDraft}
          loadAll={loadAll}
          setError={setError}
        />
      )}
    </div>
  );
}
