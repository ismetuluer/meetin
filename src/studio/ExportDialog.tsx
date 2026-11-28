import { useEffect, useRef, useState } from "react";
import { t, type TranslationKey } from "../i18n";

export type ExportFormat = "meetin" | "csv" | "ahk";

/** Neyin dışa aktarılacağı (R3.4): bütün kitaplık ya da seçili ifadeler. */
export type ExportRequest =
  | { scope: "library" }
  | { scope: "subset"; ids: number[]; fileName: string; title: string };

const FORMATS: ReadonlyArray<{
  id: ExportFormat;
  label: TranslationKey;
  desc: TranslationKey;
}> = [
  { id: "meetin", label: "studio.exportMeetin", desc: "export.meeDesc" },
  { id: "csv", label: "studio.exportCsv", desc: "export.csvDesc" },
  { id: "ahk", label: "studio.exportAhk", desc: "export.ahkDesc" },
];

interface ExportDialogProps {
  request: ExportRequest;
  libraryCount: number;
  onExport: (format: ExportFormat) => Promise<void>;
  onClose: () => void;
}

/** Tek "Dışa aktar…" penceresi: kapsam yazılır, biçim burada seçilir (R3.4). */
export function ExportDialog({
  request,
  libraryCount,
  onExport,
  onClose,
}: ExportDialogProps) {
  const [format, setFormat] = useState<ExportFormat>("meetin");
  const [busy, setBusy] = useState(false);
  const firstRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    firstRef.current?.focus();
  }, []);

  const scopeText =
    request.scope === "library"
      ? t("export.scopeLibrary", { count: libraryCount })
      : t("export.scopeSubset", {
          title: request.title,
          count: request.ids.length,
        });

  async function submit() {
    setBusy(true);
    try {
      await onExport(format);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className="modal-backdrop"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      <div
        className="modal-card export-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-dialog-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h2 id="export-dialog-title">{t("export.title")}</h2>
          <span className="muted small">{scopeText}</span>
        </div>
        <div className="modal-body">
          <div
            className="export-format-list"
            role="radiogroup"
            aria-label={t("export.formatLabel")}
          >
            {FORMATS.map((f, i) => (
              <label
                key={f.id}
                className={`export-format ${format === f.id ? "is-selected" : ""}`}
              >
                <input
                  ref={i === 0 ? firstRef : undefined}
                  type="radio"
                  name="export-format"
                  value={f.id}
                  checked={format === f.id}
                  onChange={() => setFormat(f.id)}
                />
                <span className="export-format-text">
                  <strong>{t(f.label)}</strong>
                  <span className="muted small">{t(f.desc)}</span>
                </span>
              </label>
            ))}
          </div>
        </div>
        <div className="modal-footer">
          <button type="button" className="btn btn-sm" onClick={onClose}>
            {t("form.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={busy}
            onClick={() => void submit()}
          >
            {t("export.submit")}
          </button>
        </div>
      </div>
    </div>
  );
}
