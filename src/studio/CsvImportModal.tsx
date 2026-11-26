import { useState, type ChangeEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type {
  ConflictStrategy,
  CsvColumnMapping,
  CsvImportSummary,
  CsvTablePreview,
} from "./types";

interface CsvImportModalProps {
  onClose: () => void;
  onImported: () => void;
}

export function CsvImportModal({ onClose, onImported }: CsvImportModalProps) {
  const [rawBytes, setRawBytes] = useState<number[] | null>(null);
  const [fileName, setFileName] = useState<string>("");
  const [preview, setPreview] = useState<CsvTablePreview | null>(null);
  const [mapping, setMapping] = useState<CsvColumnMapping>({
    abbr_col: 0,
    content_col: 1,
    description_col: null,
    folder_col: null,
    tags_col: null,
    has_header: true,
  });
  const [strategy, setStrategy] = useState<ConflictStrategy>("skip");
  const [drySummary, setDrySummary] = useState<CsvImportSummary | null>(null);
  const [doneSummary, setDoneSummary] = useState<CsvImportSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleFileChange(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setError(null);
    setDrySummary(null);
    setDoneSummary(null);
    setFileName(file.name);

    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      setRawBytes(bytes);
      const prev = await invoke<CsvTablePreview>("parse_csv_preview", { bytes });
      setPreview(prev);
      setMapping({
        abbr_col: 0,
        content_col: prev.headers.length > 1 ? 1 : 0,
        description_col: prev.headers.length > 2 ? 2 : null,
        folder_col: prev.headers.length > 3 ? 3 : null,
        tags_col: prev.headers.length > 4 ? 4 : null,
        has_header: true,
      });
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    }
  }

  async function runImport(dryRun: boolean) {
    if (!rawBytes) return;
    setBusy(true);
    setError(null);
    try {
      const summary = await invoke<CsvImportSummary>("import_csv_phrases", {
        bytes: rawBytes,
        mapping,
        strategy,
        dryRun,
      });
      if (dryRun) {
        setDrySummary(summary);
      } else {
        setDoneSummary(summary);
        onImported();
      }
    } catch (err) {
      setError(t("error.generic", { message: String(err) }));
    } finally {
      setBusy(false);
    }
  }

  const colOptions = (preview?.headers ?? []).map((h, i) => ({
    index: i,
    label: `${i + 1}: ${h || `Sütun ${i + 1}`}`,
  }));

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal-card">
        <div className="modal-header">
          <h2>{t("csv.title")}</h2>
          <button type="button" className="btn" onClick={onClose}>
            {t("csv.close")}
          </button>
        </div>

        <div className="modal-body">
          <div className="form-row">
            <label className="btn">
              {t("csv.chooseFile")}
              <input
                type="file"
                accept=".csv,text/csv"
                onChange={handleFileChange}
                style={{ display: "none" }}
              />
            </label>
            {fileName && <span className="muted">{fileName}</span>}
          </div>

          {preview && (
            <>
              <div className="csv-meta">
                <span>
                  {t("csv.encoding")}: <strong>{preview.detected_encoding}</strong>
                </span>
                <span>
                  {t("csv.delimiter")}: <strong>{preview.detected_delimiter}</strong>
                </span>
                <span>
                  {t("csv.totalRows")}: <strong>{preview.total_rows}</strong>
                </span>
              </div>

              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={mapping.has_header}
                  onChange={(e) =>
                    setMapping({ ...mapping, has_header: e.target.checked })
                  }
                />
                {t("csv.hasHeader")}
              </label>

              <div className="mapping-grid">
                <div>
                  <label>{t("csv.colAbbr")}</label>
                  <select
                    className="select"
                    value={mapping.abbr_col}
                    onChange={(e) =>
                      setMapping({ ...mapping, abbr_col: Number(e.target.value) })
                    }
                  >
                    {colOptions.map((c) => (
                      <option key={c.index} value={c.index}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label>{t("csv.colContent")}</label>
                  <select
                    className="select"
                    value={mapping.content_col}
                    onChange={(e) =>
                      setMapping({
                        ...mapping,
                        content_col: Number(e.target.value),
                      })
                    }
                  >
                    {colOptions.map((c) => (
                      <option key={c.index} value={c.index}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label>{t("csv.colDesc")}</label>
                  <select
                    className="select"
                    value={mapping.description_col ?? ""}
                    onChange={(e) =>
                      setMapping({
                        ...mapping,
                        description_col:
                          e.target.value === "" ? null : Number(e.target.value),
                      })
                    }
                  >
                    <option value="">{t("csv.none")}</option>
                    {colOptions.map((c) => (
                      <option key={c.index} value={c.index}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label>{t("csv.colFolder")}</label>
                  <select
                    className="select"
                    value={mapping.folder_col ?? ""}
                    onChange={(e) =>
                      setMapping({
                        ...mapping,
                        folder_col:
                          e.target.value === "" ? null : Number(e.target.value),
                      })
                    }
                  >
                    <option value="">{t("csv.none")}</option>
                    {colOptions.map((c) => (
                      <option key={c.index} value={c.index}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label>{t("csv.colTags")}</label>
                  <select
                    className="select"
                    value={mapping.tags_col ?? ""}
                    onChange={(e) =>
                      setMapping({
                        ...mapping,
                        tags_col:
                          e.target.value === "" ? null : Number(e.target.value),
                      })
                    }
                  >
                    <option value="">{t("csv.none")}</option>
                    {colOptions.map((c) => (
                      <option key={c.index} value={c.index}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label>{t("csv.conflictStrategy")}</label>
                  <select
                    className="select"
                    value={strategy}
                    onChange={(e) =>
                      setStrategy(e.target.value as ConflictStrategy)
                    }
                  >
                    <option value="skip">{t("csv.strategySkip")}</option>
                    <option value="overwrite">{t("csv.strategyOverwrite")}</option>
                    <option value="suggest_new">
                      {t("csv.strategySuggestNew")}
                    </option>
                  </select>
                </div>
              </div>

              {drySummary && (
                <div className="info-banner">
                  {t("csv.summaryText", {
                    valid: drySummary.total_valid,
                    imported: drySummary.imported,
                    overwritten: drySummary.overwritten,
                    renamed: drySummary.renamed,
                    skipped: drySummary.skipped,
                    conflicts: drySummary.conflicts.length,
                  })}
                </div>
              )}

              {doneSummary && (
                <div className="success-banner">
                  {t("csv.doneText", {
                    imported: doneSummary.imported,
                    overwritten: doneSummary.overwritten,
                    renamed: doneSummary.renamed,
                    skipped: doneSummary.skipped,
                  })}
                </div>
              )}
            </>
          )}

          {error && <div className="error-banner">{error}</div>}
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="btn"
            disabled={!rawBytes || busy}
            onClick={() => runImport(true)}
          >
            {t("csv.previewBtn")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={!rawBytes || busy}
            onClick={() => runImport(false)}
          >
            {t("csv.importBtn")}
          </button>
        </div>
      </div>
    </div>
  );
}
