import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { formatRelativeDate } from "./studioHelpers";
import { VarPreview } from "./VarPreview";
import type { ClipboardHistoryRecord, PhraseRecord, UsageStats } from "./types";

type SetMessage = Dispatch<SetStateAction<string | null>>;

interface ClipboardListProps {
  items: ClipboardHistoryRecord[];
  openNewDraft: (prefillContent?: string, prefillHtml?: string | null) => void;
  setError: SetMessage;
  setNotice: SetMessage;
}

/** Pano geçmişi görünümü: ifadeye çevirme ve harf dönüşümüyle kopyalama. */
export function ClipboardList({ items, openNewDraft, setError, setNotice }: ClipboardListProps) {
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

  if (items.length === 0) {
    return (
      <div className="empty-state">
        <p className="muted small">{t("studio.clipboardEmpty")}</p>
      </div>
    );
  }
  return (
    <>
      {items.map((item) => (
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
      ))}
    </>
  );
}

/** Kullanım istatistikleri görünümü: bugün, toplam ve en çok kullanılanlar. */
export function StatsList({ stats }: { stats: UsageStats | null }) {
  return (
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
  );
}

interface TrashListProps {
  phrases: PhraseRecord[];
  loadAll: () => Promise<void>;
  setError: SetMessage;
}

/** Çöp kutusu görünümü: geri yükleme ve kalıcı silme. */
export function TrashList({ phrases, loadAll, setError }: TrashListProps) {
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

  if (phrases.length === 0) {
    return (
      <div className="empty-state">
        <p className="muted small">{t("studio.trashEmpty")}</p>
      </div>
    );
  }
  return (
    <>
      {phrases.map((p) => (
        <div key={p.id} className="phrase-list-row">
          <div className="row-top-line">
            <span className="abbr-chip">{p.abbr}</span>
            <strong className="row-title">
              {p.description || <VarPreview text={p.content} />}
            </strong>
          </div>
          <div className="row-preview-line">
            <VarPreview text={p.content} />
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
      ))}
    </>
  );
}
