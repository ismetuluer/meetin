import type { Dispatch, MouseEvent as ReactMouseEvent, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { ExportRequest } from "./ExportDialog";
import { actionIds, clickRow, type ListSelection } from "./listSelection";
import { previewText } from "./varTokens";
import { VarPreview } from "./VarPreview";
import { ClipboardList, StatsList, TrashList } from "./StudioSideViews";
import {
  exportFileBase,
  formatRelativeDate,
  type ContextMenuState,
  type SidebarView,
  type SortMode,
} from "./studioHelpers";
import type { ClipboardHistoryRecord, PhraseRecord, UsageStats } from "./types";

type SetMessage = Dispatch<SetStateAction<string | null>>;

interface StudioPhraseListProps {
  view: SidebarView;
  centerTitle: string;
  trashCount: number;
  clipboardCount: number;
  displayedPhrases: PhraseRecord[];
  displayedOrder: number[];
  filteredClipboard: ClipboardHistoryRecord[];
  stats: UsageStats | null;
  selection: ListSelection;
  setSelection: Dispatch<SetStateAction<ListSelection>>;
  query: string;
  setQuery: Dispatch<SetStateAction<string>>;
  sortMode: SortMode;
  setSortMode: Dispatch<SetStateAction<SortMode>>;
  swipedId: number | null;
  setSwipedId: Dispatch<SetStateAction<number | null>>;
  openDraftId: number | null;
  startEdit: (p: PhraseRecord) => void;
  openNewDraft: (prefillContent?: string, prefillHtml?: string | null) => void;
  openExport: (request: ExportRequest) => void;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState>>;
  handleSoftDelete: (id: number) => Promise<void>;
  loadAll: () => Promise<void>;
  setError: SetMessage;
  setNotice: SetMessage;
}

/** Orta sütun: başlık, toplu seçim çubuğu, arama/sıralama ve ifade listesi. */
export function StudioPhraseList({
  view,
  centerTitle,
  trashCount,
  clipboardCount,
  displayedPhrases,
  displayedOrder,
  filteredClipboard,
  stats,
  selection,
  setSelection,
  query,
  setQuery,
  sortMode,
  setSortMode,
  swipedId,
  setSwipedId,
  openDraftId,
  startEdit,
  openNewDraft,
  openExport,
  setContextMenu,
  handleSoftDelete,
  loadAll,
  setError,
  setNotice,
}: StudioPhraseListProps) {
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

  /** Tık / Ctrl+tık / Shift+tık (R3.1, `listSelection.ts`) */
  function handleRowClick(p: PhraseRecord, e: ReactMouseEvent) {
    const r = clickRow(
      selection,
      displayedOrder,
      p.id,
      { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
      openDraftId,
    );
    setSelection(r.sel);
    if (r.open) {
      setSwipedId(null);
      startEdit(p);
    }
  }

  return (
    <section className="fluent-list-col">
      <div className="list-col-head">
        <h2>{centerTitle}</h2>
        {view.kind === "trash" ? (
          trashCount > 0 && (
            <button
              type="button"
              className="btn btn-danger btn-sm"
              onClick={() => void handleEmptyTrash()}
            >
              {t("studio.emptyTrash")}
            </button>
          )
        ) : view.kind === "clipboard" ? (
          clipboardCount > 0 && (
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
              <button
                type="button"
                className="icon-btn"
                title={t("studio.exportListBtn")}
                aria-label={t("studio.exportListBtn")}
                onClick={() =>
                  openExport({
                    scope: "subset",
                    ids: displayedOrder,
                    fileName: exportFileBase(centerTitle),
                    title: centerTitle,
                  })
                }
              >
                <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path d="M8 2.5V10M4.5 6.5L8 10L11.5 6.5M3 13H13" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
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
          <ClipboardList
            items={filteredClipboard}
            openNewDraft={openNewDraft}
            setError={setError}
            setNotice={setNotice}
          />
        ) : view.kind === "stats" ? (
          <StatsList stats={stats} />
        ) : view.kind === "trash" ? (
          <TrashList phrases={displayedPhrases} loadAll={loadAll} setError={setError} />
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
            const isOpen = openDraftId === p.id;
            // Çoklu seçimde yalnız seçilenler vurgulanır; açık satır sol çizgisini korur
            const isSelected =
              selection.ids.length > 0 ? selection.ids.includes(p.id) : isOpen;
            const isSwiped = swipedId === p.id;
            const title =
              p.description.trim() || previewText(p.content);
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
                    <VarPreview text={p.content} />
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </section>
  );
}
