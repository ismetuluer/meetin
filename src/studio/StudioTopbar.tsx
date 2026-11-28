import { useRef, type ChangeEvent, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { KbdGroup } from "../design/Kbd";
import { MeetinSheep } from "../mascot/MeetinSheep";
import type { PageMode } from "./studioHelpers";
import type { EngineStatus, ImportedPhraseItem } from "./types";

interface StudioTopbarProps {
  pageMode: PageMode;
  status: EngineStatus | null;
  showMoreMenu: boolean;
  setShowMoreMenu: Dispatch<SetStateAction<boolean>>;
  setPageMode: Dispatch<SetStateAction<PageMode>>;
  setShowPalette: Dispatch<SetStateAction<boolean>>;
  setShowBulkModal: Dispatch<SetStateAction<boolean>>;
  setShowCsv: Dispatch<SetStateAction<boolean>>;
  setImportPreview: (preview: { items: ImportedPhraseItem[]; filename: string }) => void;
  setError: Dispatch<SetStateAction<string | null>>;
  onTogglePause: () => Promise<void>;
  onNewFromClipboard: () => Promise<void>;
  onOpenLibraryExport: () => void;
}

/** Üst çubuk: maskot, motor durumu, komut paleti ve "daha fazla" menüsü. */
export function StudioTopbar({
  pageMode,
  status,
  showMoreMenu,
  setShowMoreMenu,
  setPageMode,
  setShowPalette,
  setShowBulkModal,
  setShowCsv,
  setImportPreview,
  setError,
  onTogglePause,
  onNewFromClipboard,
  onOpenLibraryExport,
}: StudioTopbarProps) {
  const fileInputRef = useRef<HTMLInputElement | null>(null);

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

  return (
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
              onClick={() => void onTogglePause()}
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
                  void onNewFromClipboard();
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
              <button type="button" onClick={onOpenLibraryExport}>
                {t("export.menuEntry")}
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
  );
}
