import type { Dispatch, FormEvent, SetStateAction } from "react";
import { t } from "../i18n";
import {
  extractDraggedIds,
  type ContextMenuState,
  type SidebarView,
} from "./studioHelpers";
import type { FolderRecord, PhraseRecord, UsageStats } from "./types";

interface StudioSidebarProps {
  sidebarCollapsed: boolean;
  setSidebarCollapsed: Dispatch<SetStateAction<boolean>>;
  view: SidebarView;
  setView: Dispatch<SetStateAction<SidebarView>>;
  folders: FolderRecord[];
  activePhrases: PhraseRecord[];
  trashCount: number;
  clipboardCount: number;
  stats: UsageStats | null;
  showFolderInput: boolean;
  setShowFolderInput: Dispatch<SetStateAction<boolean>>;
  newFolderName: string;
  setNewFolderName: Dispatch<SetStateAction<string>>;
  dropFolderId: number | null;
  setDropFolderId: Dispatch<SetStateAction<number | null>>;
  dropTrash: boolean;
  setDropTrash: Dispatch<SetStateAction<boolean>>;
  renamingFolderId: number | null;
  setRenamingFolderId: Dispatch<SetStateAction<number | null>>;
  renameFolderDraft: string;
  setRenameFolderDraft: Dispatch<SetStateAction<string>>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState>>;
  handleCreateFolder: (e: FormEvent) => Promise<void>;
  handleFolderDrop: (folderId: number, ids: number[], copy: boolean) => Promise<void>;
  handleBulkDelete: (ids: number[]) => Promise<void>;
  handleUpdateFolder: (id: number, name: string | null, color: string | null) => Promise<void>;
  handleDeleteFolder: (id: number) => Promise<void>;
  onOpenHelp: () => void;
  onOpenSettings: () => void;
}

/** Sol kenar çubuğu: görünümler, klasörler (sürükle-bırak hedefi), çöp kutusu ve alt bağlantılar. */
export function StudioSidebar({
  sidebarCollapsed,
  setSidebarCollapsed,
  view,
  setView,
  folders,
  activePhrases,
  trashCount,
  clipboardCount,
  stats,
  showFolderInput,
  setShowFolderInput,
  newFolderName,
  setNewFolderName,
  dropFolderId,
  setDropFolderId,
  dropTrash,
  setDropTrash,
  renamingFolderId,
  setRenamingFolderId,
  renameFolderDraft,
  setRenameFolderDraft,
  setContextMenu,
  handleCreateFolder,
  handleFolderDrop,
  handleBulkDelete,
  handleUpdateFolder,
  handleDeleteFolder,
  onOpenHelp,
  onOpenSettings,
}: StudioSidebarProps) {
  return (
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
            <span className="nav-count">{clipboardCount}</span>
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
                title={`${folder.name} · ${t("studio.folderHint")}`}
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
            <span className="nav-count">{trashCount}</span>
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
          onClick={onOpenHelp}
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
          onClick={onOpenSettings}
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
  );
}
