import { useMemo, useRef, type ChangeEvent, type Dispatch, type SetStateAction } from "react";
import { t } from "../i18n";
import { StackPanel } from "../design/StackPanel";
import { TagInput } from "../design/TagInput";
import { RichEditor } from "./RichEditor";
import { plainTextToRichHtml } from "./richHtml";
import {
  buildEmbeddedImageTag,
  extractEmbeddedImages,
  isSafeEmbeddedImageDataUrl,
  isSupportedImageFile,
  MAX_EMBEDDED_IMAGE_BYTES,
  removeEmbeddedImageAt,
} from "./richImages";
import { VarPreview } from "./VarPreview";
import { StudioEditorFields } from "./StudioEditorFields";
import { StudioRules } from "./StudioRules";
import {
  deriveAutoDescription,
  formatRelativeDate,
  formatVersionTime,
  VARIABLE_SNIPPETS,
} from "./studioHelpers";
import type { StudioEditorState } from "./useStudioEditor";
import type { PhraseRecord } from "./types";

type SetMessage = Dispatch<SetStateAction<string | null>>;

interface StudioEditorProps {
  editor: StudioEditorState;
  selectedPhrase: PhraseRecord | null;
  handleSoftDelete: (id: number) => Promise<void>;
  setError: SetMessage;
  setNotice: SetMessage;
}

/** Sağ sütun: açık ifadenin düzenleyicisi ya da boş durum. */
export function StudioEditor({
  editor,
  selectedPhrase,
  handleSoftDelete,
  setError,
  setNotice,
}: StudioEditorProps) {
  const {
    draft,
    setDraft,
    descriptionManual,
    setDescriptionManual,
    autoSaveState,
    showHistoryDrawer,
    setShowHistoryDrawer,
    versions,
    editorSessionId,
    folders,
    openNewDraft,
    handleSubmit,
    handleDuplicate,
    handleRestoreVersion,
  } = editor;
  const imageInputRef = useRef<HTMLInputElement | null>(null);

  const embeddedImages = useMemo(
    () => extractEmbeddedImages(draft?.content_html ?? null),
    [draft?.content_html],
  );

  function handleInsertImageFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file || !draft) return;
    setError(null);
    setNotice(null);

    if (!isSupportedImageFile(file)) {
      setError(t("studio.imageUnsupportedType"));
      return;
    }
    if (file.size > MAX_EMBEDDED_IMAGE_BYTES) {
      setError(t("studio.imageTooLarge"));
      return;
    }

    const reader = new FileReader();
    reader.onerror = () => {
      setError(t("studio.imageReadError"));
    };
    reader.onload = () => {
      const dataUrl = typeof reader.result === "string" ? reader.result : "";
      if (!isSafeEmbeddedImageDataUrl(dataUrl)) {
        setError(t("studio.imageUnsupportedType"));
        return;
      }
      const imgTag = buildEmbeddedImageTag(
        dataUrl,
        file.name || t("studio.imageAltFallback"),
      );
      setDraft((prev) => {
        if (!prev) return prev;
        const baseHtml =
          prev.content_html ?? (prev.content ? plainTextToRichHtml(prev.content) : "");
        const nextHtml = baseHtml ? `${baseHtml}<br>${imgTag}` : imgTag;
        const nextContent = prev.content.trim()
          ? prev.content
          : `[${t("studio.imageAltFallback")}: ${file.name}]`;
        return {
          ...prev,
          content: nextContent,
          content_html: nextHtml,
          description: descriptionManual
            ? prev.description
            : prev.description.trim() || deriveAutoDescription(nextContent),
        };
      });
      setNotice(t("studio.imageAddedNotice"));
    };
    reader.readAsDataURL(file);
  }

  return (
    <main className="fluent-detail-col">
      {!draft ? (
        <div className="empty-state" style={{ margin: "auto" }}>
          <p className="muted">{t("studio.selectOrCreate")}</p>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => openNewDraft()}
          >
            + {t("toolbar.newPhrase")}
          </button>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="detail-canvas">
          <div className="detail-header-row">
            <div className="detail-title-wrap">
              <span className="tiny-label">{t("studio.descriptionTitle")}</span>
              <input
                className="detail-title-input"
                value={draft.description}
                onChange={(e) => {
                  setDescriptionManual(true);
                  setDraft({ ...draft, description: e.target.value });
                }}
                placeholder={t("form.descriptionPlaceholder")}
              />
            </div>
            <div className="row-actions" style={{ alignItems: "center" }}>
              {autoSaveState !== "idle" && (
                <span
                  className={`autosave-status is-${autoSaveState}`}
                  role="status"
                  title={autoSaveState === "saving" ? t("studio.autoSaving") : t("studio.autoSaved")}
                  aria-label={autoSaveState === "saving" ? t("studio.autoSaving") : t("studio.autoSaved")}
                >
                  {autoSaveState === "saved" && (
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                      <path d="M3.5 8.5L6.5 11.5L12.5 4.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  )}
                </span>
              )}
              {draft.id !== null && (
                <button
                  type="button"
                  className="icon-btn"
                  title={t("studio.duplicate")}
                  aria-label={t("studio.duplicate")}
                  onClick={() => void handleDuplicate()}
                >
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                    <rect x="5.5" y="5.5" width="8" height="8" rx="1.5" stroke="currentColor" strokeWidth="1.4" />
                    <path d="M10.5 3.5V3A1.5 1.5 0 0 0 9 1.5H4A1.5 1.5 0 0 0 2.5 3V8A1.5 1.5 0 0 0 4 9.5H4.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                  </svg>
                </button>
              )}
              {draft.id !== null && (
                <button
                  type="button"
                  title={t("tip.historyToggle")}
                  className={`btn btn-sm ${showHistoryDrawer ? "is-active-outline" : ""}`}
                  onClick={() => setShowHistoryDrawer((v) => !v)}
                >
                  {t("studio.historyPill", { count: versions.length })}
                </button>
              )}
            </div>
          </div>

          <TagInput
            value={draft.tags}
            onChange={(tags) => setDraft((d) => (d ? { ...d, tags } : d))}
            placeholder={t("studio.addTagPlaceholder")}
            inputLabel={t("studio.tagInputLabel")}
            removeLabel={(tag) => t("studio.removeTag", { tag })}
          />

          <StackPanel
            open={showHistoryDrawer}
            items={versions.map((ver) => ({
              id: ver.id,
              meta: formatVersionTime(ver.created_at),
              preview: <VarPreview text={ver.content} />,
            }))}
            title={t("studio.tabHistory")}
            closeLabel={t("studio.closeHistory")}
            emptyText={t("studio.historyEmpty")}
            actionLabel={t("studio.restoreVersion")}
            onAction={(id) => void handleRestoreVersion(id)}
            onClose={() => setShowHistoryDrawer(false)}
          />

          <div className="content-card-section">
            <span className="tiny-label">{t("studio.tabContent")}</span>
            <div className="unified-content-card" onClick={(e) => e.stopPropagation()}>
              <RichEditor
                draftKey={`${editorSessionId}-${draft.id ?? "new"}`}
                content={draft.content}
                contentHtml={draft.content_html}
                placeholder={t("form.contentPlaceholder")}
                variableSnippets={VARIABLE_SNIPPETS}
                onRequestInsertImage={() => imageInputRef.current?.click()}
                onChange={(nextContent, nextHtml) => {
                  setDraft((prev) =>
                    prev
                      ? {
                          ...prev,
                          content: nextContent,
                          content_html: nextHtml,
                          description: descriptionManual
                            ? prev.description
                            : deriveAutoDescription(nextContent),
                        }
                      : prev,
                  );
                }}
              />
              <input
                ref={imageInputRef}
                type="file"
                accept="image/png,image/jpeg,image/gif,image/webp,image/svg+xml"
                style={{ display: "none" }}
                onChange={handleInsertImageFile}
              />

              {embeddedImages.length > 0 && (
                <div className="rich-image-preview-strip">
                  <span className="muted small">
                    {t("studio.embeddedImages", {
                      count: embeddedImages.length,
                    })}
                  </span>
                  <div className="rich-image-grid">
                    {embeddedImages.map((img) => (
                      <div key={img.key} className="rich-image-card">
                        <img
                          src={img.src}
                          alt={img.alt || t("studio.imageAltFallback")}
                          className="rich-image-thumb"
                        />
                        <button
                          type="button"
                          className="rich-image-remove"
                          title={t("studio.removeImage")}
                          aria-label={t("studio.removeImage")}
                          onClick={() =>
                            setDraft((prev) =>
                              prev && prev.content_html
                                ? {
                                    ...prev,
                                    content_html: removeEmbeddedImageAt(
                                      prev.content_html,
                                      img.matchStart,
                                      img.matchEnd,
                                    ),
                                  }
                                : prev,
                            )
                          }
                        >
                          ×
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>

          <StudioEditorFields
            draft={draft}
            setDraft={setDraft}
            folders={folders}
            showCaseMenu={editor.showCaseMenu}
            setShowCaseMenu={editor.setShowCaseMenu}
          />

          <StudioRules
            draftId={draft.id}
            appRules={editor.appRules}
            setAppRules={editor.setAppRules}
            runningApps={editor.runningApps}
            newRuleProc={editor.newRuleProc}
            setNewRuleProc={editor.setNewRuleProc}
            newRuleKind={editor.newRuleKind}
            setNewRuleKind={editor.setNewRuleKind}
            showRuleAdd={editor.showRuleAdd}
            setShowRuleAdd={editor.setShowRuleAdd}
            refreshRunningApps={editor.refreshRunningApps}
            setError={setError}
          />

          {selectedPhrase && (
            <div className="usage-cards-row">
              <div className="usage-stat-card">
                <strong>{selectedPhrase.use_count}</strong>
                <span className="muted small">{t("studio.usedTimes")}</span>
              </div>
              <div className="usage-stat-card">
                <strong>
                  {formatRelativeDate(selectedPhrase.last_used_at)}
                </strong>
                <span className="muted small">{t("studio.lastUsedLabel")}</span>
              </div>
            </div>
          )}

          <div className="detail-footer-actions">
            {draft.id !== null && (
              <label className="checkbox-inline" style={{ marginRight: "auto" }}>
                <input
                  type="checkbox"
                  checked={draft.enabled}
                  onChange={(e) =>
                    setDraft({ ...draft, enabled: e.target.checked })
                  }
                />
                <span>{t("studio.enabledLabel")}</span>
              </label>
            )}
            {draft.id !== null && (
              <button
                type="button"
                className="btn btn-danger btn-sm"
                onClick={() => void handleSoftDelete(draft.id!)}
              >
                {t("list.delete")}
              </button>
            )}
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => setDraft(null)}
            >
              {t("form.cancel")}
            </button>
            <button type="submit" className="btn btn-primary btn-sm">
              {t("form.save")}
            </button>
          </div>
        </form>
      )}
    </main>
  );
}
