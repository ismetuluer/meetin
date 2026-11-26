import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { suggestAbbreviation } from "../island/abbrSuggest";
import type {
  ConflictStrategy,
  CsvImportSummary,
  FolderRecord,
  ImportedPhraseItem,
} from "./types";

interface BulkAddModalProps {
  folders: FolderRecord[];
  initialItems?: ImportedPhraseItem[];
  sourceTitle?: string;
  onClose: () => void;
  onDone: (summary: CsvImportSummary) => void;
}

interface RowDraft {
  id: number;
  abbr: string;
  content: string;
  folder: string;
  description: string;
  tags: string[];
  case_mode: ImportedPhraseItem["case_mode"];
  content_html: string | null;
}

export function BulkAddModal({
  folders,
  initialItems,
  sourceTitle,
  onClose,
  onDone,
}: BulkAddModalProps) {
  const [rows, setRows] = useState<RowDraft[]>(() => {
    if (initialItems && initialItems.length > 0) {
      return initialItems.map((item, idx) => ({
        id: idx + 1,
        abbr: item.abbr,
        content: item.content,
        folder: item.folder ?? "",
        description: item.description,
        tags: item.tags,
        case_mode: item.case_mode,
        content_html: item.content_html,
      }));
    }
    return [
      {
        id: 1,
        abbr: "",
        content: "",
        folder: "",
        description: "",
        tags: [],
        case_mode: "match_typed",
        content_html: null,
      },
      {
        id: 2,
        abbr: "",
        content: "",
        folder: "",
        description: "",
        tags: [],
        case_mode: "match_typed",
        content_html: null,
      },
      {
        id: 3,
        abbr: "",
        content: "",
        folder: "",
        description: "",
        tags: [],
        case_mode: "match_typed",
        content_html: null,
      },
    ];
  });
  const [strategy, setStrategy] = useState<ConflictStrategy>("suggest_new");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const addRow = () => {
    setRows((prev) => [
      ...prev,
      {
        id: (prev[prev.length - 1]?.id ?? 0) + 1,
        abbr: "",
        content: "",
        folder: prev[prev.length - 1]?.folder ?? "",
        description: "",
        tags: [],
        case_mode: "match_typed",
        content_html: null,
      },
    ]);
  };

  const updateRow = (id: number, patch: Partial<RowDraft>) => {
    setRows((prev) => prev.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  };

  const removeRow = (id: number) => {
    setRows((prev) => (prev.length <= 1 ? prev : prev.filter((r) => r.id !== id)));
  };

  const handleSubmit = async () => {
    setError(null);
    const items: ImportedPhraseItem[] = rows
      .filter((r) => r.content.trim().length > 0)
      .map((r) => ({
        abbr: (r.abbr.trim() || suggestAbbreviation(r.content)).trim(),
        content: r.content,
        content_html: r.content_html,
        description: r.description,
        folder: r.folder.trim() || null,
        tags: r.tags,
        case_mode: r.case_mode,
      }))
      .filter((i) => i.abbr.length > 0);

    if (items.length === 0) {
      setError(t("error.requiredFields"));
      return;
    }

    setBusy(true);
    try {
      const summary = await invoke<CsvImportSummary>("bulk_create_phrases", {
        items,
        conflictStrategy: strategy,
      });
      onDone(summary);
    } catch (e) {
      setError(t("error.generic", { message: String(e) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-card modal-card-wide">
        <div className="modal-header">
          <h2>
            {sourceTitle
              ? `${t("bulk.importPreviewTitle")}: ${sourceTitle}`
              : t("bulk.title")}
          </h2>
          <button type="button" className="btn" onClick={onClose}>
            {t("csv.close")}
          </button>
        </div>

        <div className="modal-body">
          <p className="muted small">{t("bulk.hint")}</p>
          {error && <div className="error-banner">{error}</div>}

          <div className="bulk-table-wrap">
            <table className="phrase-table">
              <thead>
                <tr>
                  <th style={{ width: "160px" }}>{t("bulk.abbrCol")}</th>
                  <th>{t("bulk.contentCol")}</th>
                  <th style={{ width: "160px" }}>{t("bulk.folderCol")}</th>
                  <th style={{ width: "54px" }} />
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <input
                        className="input"
                        value={row.abbr}
                        onChange={(e) => updateRow(row.id, { abbr: e.target.value })}
                        placeholder={
                          row.content.trim()
                            ? suggestAbbreviation(row.content)
                            : t("form.abbrPlaceholder")
                        }
                      />
                    </td>
                    <td>
                      <textarea
                        className="textarea"
                        rows={2}
                        value={row.content}
                        onChange={(e) =>
                          updateRow(row.id, { content: e.target.value })
                        }
                        placeholder={t("form.contentPlaceholder")}
                      />
                    </td>
                    <td>
                      <input
                        className="input"
                        list="bulk-folder-options"
                        value={row.folder}
                        onChange={(e) =>
                          updateRow(row.id, { folder: e.target.value })
                        }
                        placeholder={t("add.noFolder")}
                      />
                    </td>
                    <td>
                      <button
                        type="button"
                        className="btn btn-danger"
                        onClick={() => removeRow(row.id)}
                        disabled={rows.length <= 1}
                        title={t("list.delete")}
                      >
                        ×
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <datalist id="bulk-folder-options">
              {folders.map((f) => (
                <option key={f.id} value={f.name} />
              ))}
            </datalist>
          </div>

          <div className="form-row" style={{ justifyContent: "space-between" }}>
            <button type="button" className="btn" onClick={addRow}>
              {t("bulk.addRow")}
            </button>

            <label className="form-row">
              <span className="muted small">{t("csv.conflictStrategy")}</span>
              <select
                className="select"
                value={strategy}
                onChange={(e) => setStrategy(e.target.value as ConflictStrategy)}
              >
                <option value="suggest_new">{t("csv.strategySuggestNew")}</option>
                <option value="overwrite">{t("csv.strategyOverwrite")}</option>
                <option value="skip">{t("csv.strategySkip")}</option>
              </select>
            </label>
          </div>
        </div>

        <div className="modal-footer">
          <button type="button" className="btn" onClick={onClose}>
            {t("form.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void handleSubmit()}
          >
            {t("bulk.saveAll")} ({rows.filter((r) => r.content.trim()).length})
          </button>
        </div>
      </div>
    </div>
  );
}
