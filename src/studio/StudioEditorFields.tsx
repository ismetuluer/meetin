import type { Dispatch, SetStateAction } from "react";
import { t } from "../i18n";
import { ShortcutField } from "../design/Kbd";
import { captureShortcutCombo } from "./shortcutCapture";
import { CASE_MODE_SYMBOLS, type EditorDraft } from "./studioHelpers";
import type { FolderRecord } from "./types";

interface StudioEditorFieldsProps {
  draft: EditorDraft;
  setDraft: Dispatch<SetStateAction<EditorDraft | null>>;
  folders: FolderRecord[];
  showCaseMenu: boolean;
  setShowCaseMenu: Dispatch<SetStateAction<boolean>>;
}

/** Kısaltma + harf modu, kısayol tuşu ve klasör alanı (R3.2: klasör ayrı alan). */
export function StudioEditorFields({
  draft,
  setDraft,
  folders,
  showCaseMenu,
  setShowCaseMenu,
}: StudioEditorFieldsProps) {
  return (
    <>
      <div className="meta-two-col">
        <div className="meta-field-group">
          <span className="tiny-label">{t("form.abbrLabel")}</span>
          <div className="unified-abbr-box" onClick={(e) => e.stopPropagation()}>
            <input
              className="abbr-input-bold"
              value={draft.abbr}
              onChange={(e) =>
                setDraft({ ...draft, abbr: e.target.value })
              }
              placeholder={t("form.abbrPlaceholder")}
            />
            <div className="dropdown-wrap">
              <button
                type="button"
                className="case-mode-symbol-btn"
                title={`${t("form.caseModeLabel")}: ${t(CASE_MODE_SYMBOLS[draft.case_mode].labelKey)} — ${t(CASE_MODE_SYMBOLS[draft.case_mode].descKey)}`}
                onClick={() => setShowCaseMenu((v) => !v)}
              >
                {CASE_MODE_SYMBOLS[draft.case_mode].symbol}
              </button>
              {showCaseMenu && (
                <div className="dropdown-menu case-mode-popover">
                  <div className="case-popover-head">
                    {t("studio.caseModeMenuTitle")}
                  </div>
                  {(["match_typed", "keep_original", "exact_match"] as const).map(
                    (mode) => (
                      <button
                        key={mode}
                        type="button"
                        className={`case-popover-item ${draft.case_mode === mode ? "is-selected" : ""}`}
                        onClick={() => {
                          setDraft({ ...draft, case_mode: mode });
                          setShowCaseMenu(false);
                        }}
                      >
                        <span className="abbr-chip">
                          {CASE_MODE_SYMBOLS[mode].symbol}
                        </span>
                        <div className="case-popover-meta">
                          <strong>{t(CASE_MODE_SYMBOLS[mode].labelKey)}</strong>
                          <span className="muted small">
                            {t(CASE_MODE_SYMBOLS[mode].descKey)}
                          </span>
                        </div>
                      </button>
                    ),
                  )}
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="meta-field-group">
          <span className="tiny-label">{t("studio.shortcutLabel")}</span>
          <div className="unified-abbr-box">
            <ShortcutField
              value={draft.shortcut}
              label={t("studio.shortcutLabel")}
              title={t("settings.shortcutFieldTip")}
              onKeyDown={(e) => {
                if (e.key === "Backspace" || e.key === "Delete") {
                  e.preventDefault();
                  setDraft({ ...draft, shortcut: "" });
                  return;
                }
                const combo = captureShortcutCombo(e);
                if (combo) {
                  e.preventDefault();
                  setDraft({ ...draft, shortcut: combo });
                }
              }}
              placeholder="Ctrl+Alt+R"
            />
          </div>
        </div>
      </div>

      <div className="meta-field-group folder-field-group">
        <span className="tiny-label">{t("studio.folderLabel")}</span>
        <div className="folder-field-row">
          <select
            className="select folder-select"
            value=""
            onChange={(e) => {
              const v = e.target.value;
              if (!v) {
                setDraft({ ...draft, folder_id: null, folder_ids: [] });
                return;
              }
              const fid = Number(v);
              if (!draft.folder_ids.includes(fid)) {
                const nextIds = [...draft.folder_ids, fid];
                const fname = folders.find((f) => f.id === fid)?.name;
                const nextTags =
                  fname && !draft.tags.includes(fname)
                    ? [...draft.tags, fname]
                    : draft.tags;
                setDraft({
                  ...draft,
                  folder_id: nextIds[0] ?? null,
                  folder_ids: nextIds,
                  tags: nextTags,
                });
              }
            }}
            title={t("studio.folderLabel")}
            aria-label={t("studio.folderLabel")}
          >
            <option value="">
              {draft.folder_ids.length === 0
                ? t("studio.unfiled")
                : `+ ${t("studio.folderLabel")}`}
            </option>
            {folders.map((f) => (
              <option key={f.id} value={f.id}>
                {f.name}
              </option>
            ))}
          </select>
          {draft.folder_ids.length > 0 && (
            <div className="folder-chips-row">
              {draft.folder_ids.map((fid) => {
                const fObj = folders.find((f) => f.id === fid);
                const fname = fObj?.name ?? `#${fid}`;
                return (
                  <span key={fid} className="tag-pill">
                    {fObj?.color && (
                      <span className={`folder-color-dot folder-dot-${fObj.color}`} />
                    )}
                    {fname}
                    <button
                      type="button"
                      onClick={() => {
                        const nextIds = draft.folder_ids.filter(
                          (id) => id !== fid,
                        );
                        setDraft({
                          ...draft,
                          folder_id: nextIds[0] ?? null,
                          folder_ids: nextIds,
                        });
                      }}
                    >
                      ×
                    </button>
                  </span>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </>
  );
}
