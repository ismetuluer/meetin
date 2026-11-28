import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { ExportRequest } from "./ExportDialog";
import {
  exportFileBase,
  FOLDER_COLORS,
  type ContextMenuState,
  type SidebarView,
} from "./studioHelpers";
import type { FolderRecord, PhraseRecord } from "./types";

interface StudioContextMenuProps {
  contextMenu: NonNullable<ContextMenuState>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState>>;
  view: SidebarView;
  phrases: PhraseRecord[];
  activePhrases: PhraseRecord[];
  folders: FolderRecord[];
  startEdit: (p: PhraseRecord) => void;
  openExport: (request: ExportRequest) => void;
  handleFolderDrop: (folderId: number, ids: number[], copy: boolean) => Promise<void>;
  handleBulkDelete: (ids: number[]) => Promise<void>;
  handleUpdateFolder: (id: number, name: string | null, color: string | null) => Promise<void>;
  handleDeleteFolder: (id: number) => Promise<void>;
  setRenamingFolderId: Dispatch<SetStateAction<number | null>>;
  setRenameFolderDraft: Dispatch<SetStateAction<string>>;
  loadAll: () => Promise<void>;
  setError: Dispatch<SetStateAction<string | null>>;
}

/** Sağ tık menüsü: ifade satırı (tekil ya da seçim) ve klasör için. */
export function StudioContextMenu({
  contextMenu,
  setContextMenu,
  view,
  phrases,
  activePhrases,
  folders,
  startEdit,
  openExport,
  handleFolderDrop,
  handleBulkDelete,
  handleUpdateFolder,
  handleDeleteFolder,
  setRenamingFolderId,
  setRenameFolderDraft,
  loadAll,
  setError,
}: StudioContextMenuProps) {
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

  return (
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
              openExport({
                scope: "subset",
                ids: contextMenu.ids,
                fileName:
                  contextMenu.ids.length > 1
                    ? "meetin-secilenler"
                    : exportFileBase(contextMenu.phrase.abbr),
                title:
                  contextMenu.ids.length > 1
                    ? t("export.selectionTitle")
                    : contextMenu.phrase.abbr,
              })
            }
          >
            {t("export.menuEntry")}
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
              openExport({
                scope: "subset",
                ids: fIds,
                fileName: exportFileBase(f.name),
                title: f.name,
              });
            }}
          >
            {t("export.folderMenuEntry")}
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
  );
}
