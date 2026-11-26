import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ClipboardEvent,
  type KeyboardEvent,
} from "react";
import { t, useLanguage } from "../i18n";
import {
  normalizeSafeLinkUrl,
  plainTextToRichHtml,
  richHtmlToPlainText,
  sanitizeRichHtml,
  syncEditorHtml,
  toEditorInitialHtml,
} from "./richHtml";

export interface VariableSnippetItem {
  label: string;
  snippet: string;
}

export interface RichEditorProps {
  draftKey: string;
  content: string;
  contentHtml: string | null;
  placeholder: string;
  variableSnippets: ReadonlyArray<VariableSnippetItem>;
  onChange: (content: string, contentHtml: string | null) => void;
  onRequestInsertImage: () => void;
}

interface ActiveFormats {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  unorderedList: boolean;
  orderedList: boolean;
  link: boolean;
}

const EMPTY_FORMATS: ActiveFormats = {
  bold: false,
  italic: false,
  underline: false,
  unorderedList: false,
  orderedList: false,
  link: false,
};

function findAncestorAnchor(node: Node | null, root: HTMLElement | null): HTMLAnchorElement | null {
  let cur: Node | null = node;
  while (cur && cur !== root) {
    if (cur.nodeType === Node.ELEMENT_NODE && (cur as HTMLElement).tagName === "A") {
      return cur as HTMLAnchorElement;
    }
    cur = cur.parentNode;
  }
  return null;
}

function queryCommandStateSafe(cmd: string): boolean {
  try {
    return typeof document !== "undefined" && document.queryCommandState(cmd);
  } catch {
    return false;
  }
}

export function RichEditor({
  draftKey,
  content,
  contentHtml,
  placeholder,
  variableSnippets,
  onChange,
  onRequestInsertImage,
}: RichEditorProps) {
  useLanguage();

  const editorRef = useRef<HTMLDivElement | null>(null);
  const savedRangeRef = useRef<Range | null>(null);
  const lastEmittedRef = useRef<{
    draftKey: string;
    content: string;
    contentHtml: string | null;
  } | null>(null);

  const [formats, setFormats] = useState<ActiveFormats>(EMPTY_FORMATS);
  const [showVarMenu, setShowVarMenu] = useState(false);
  const [showLinkBar, setShowLinkBar] = useState(false);
  const [linkUrl, setLinkUrl] = useState("https://");
  const [linkError, setLinkError] = useState<string | null>(null);
  const [showHtmlSource, setShowHtmlSource] = useState(false);

  const saveSelection = useCallback(() => {
    const sel = window.getSelection();
    const root = editorRef.current;
    if (!sel || !root || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    if (root.contains(range.commonAncestorContainer)) {
      savedRangeRef.current = range.cloneRange();
    }
  }, []);

  const restoreSelection = useCallback((): Range | null => {
    const root = editorRef.current;
    if (!root) return null;
    root.focus();
    const sel = window.getSelection();
    if (!sel) return null;
    const saved = savedRangeRef.current;
    if (saved && root.contains(saved.commonAncestorContainer)) {
      sel.removeAllRanges();
      sel.addRange(saved);
      return saved;
    }
    const fallback = document.createRange();
    fallback.selectNodeContents(root);
    fallback.collapse(false);
    sel.removeAllRanges();
    sel.addRange(fallback);
    return fallback;
  }, []);

  const refreshFormats = useCallback(() => {
    const root = editorRef.current;
    const sel = window.getSelection();
    if (!root || !sel || sel.rangeCount === 0 || !root.contains(sel.anchorNode)) {
      return;
    }
    saveSelection();
    const anchor = findAncestorAnchor(sel.anchorNode, root);
    setFormats({
      bold: queryCommandStateSafe("bold"),
      italic: queryCommandStateSafe("italic"),
      underline: queryCommandStateSafe("underline"),
      unorderedList: queryCommandStateSafe("insertUnorderedList"),
      orderedList: queryCommandStateSafe("insertOrderedList"),
      link: anchor !== null,
    });
  }, [saveSelection]);

  // Dış değişikliklerde (başka ifade seçimi, sürüm geri yükleme, görsel ekleme/kaldırma) DOM'u senkronize et
  useEffect(() => {
    const root = editorRef.current;
    if (!root) return;
    const last = lastEmittedRef.current;
    const isSameAsEmitted =
      last !== null &&
      last.draftKey === draftKey &&
      last.content === content &&
      last.contentHtml === contentHtml;

    if (isSameAsEmitted) {
      return;
    }

    const nextHtml = toEditorInitialHtml(content, contentHtml);
    if (root.innerHTML !== nextHtml) {
      root.innerHTML = nextHtml;
    }
    lastEmittedRef.current = { draftKey, content, contentHtml };
    if (last?.draftKey !== draftKey) {
      savedRangeRef.current = null;
      setShowLinkBar(false);
      setShowVarMenu(false);
      setLinkError(null);
      setFormats(EMPTY_FORMATS);
    }
  }, [draftKey, content, contentHtml]);

  useEffect(() => {
    const onSelectionChange = () => {
      refreshFormats();
    };
    document.addEventListener("selectionchange", onSelectionChange);
    return () => {
      document.removeEventListener("selectionchange", onSelectionChange);
    };
  }, [refreshFormats]);

  const emitFromDom = useCallback(() => {
    const root = editorRef.current;
    if (!root) return;
    const synced = syncEditorHtml(root.innerHTML);
    let nextContent = synced.content;
    if (!nextContent.trim() && synced.content_html && /<img\b/i.test(synced.content_html)) {
      nextContent = `[${t("studio.imageAltFallback")}]`;
    }
    lastEmittedRef.current = {
      draftKey,
      content: nextContent,
      contentHtml: synced.content_html,
    };
    onChange(nextContent, synced.content_html);
    refreshFormats();
  }, [draftKey, onChange, refreshFormats]);

  const runExecCommand = useCallback(
    (command: "bold" | "italic" | "underline" | "insertUnorderedList" | "insertOrderedList") => {
      restoreSelection();
      try {
        document.execCommand(command, false);
      } catch {
        // Desteklenmeyen ortamda sessizce geç
      }
      emitFromDom();
    },
    [emitFromDom, restoreSelection],
  );

  const handleClearFormatting = useCallback(() => {
    const root = editorRef.current;
    if (!root) return;
    const range = restoreSelection();
    if (range && !range.collapsed) {
      try {
        document.execCommand("removeFormat", false);
        document.execCommand("unlink", false);
      } catch {
        // yok say
      }
      emitFromDom();
      return;
    }
    // Seçim yoksa tüm içeriğin zengin biçimini temizleyip düz metne döndür
    const plain = richHtmlToPlainText(root.innerHTML);
    root.innerHTML = plainTextToRichHtml(plain);
    lastEmittedRef.current = {
      draftKey,
      content: plain,
      contentHtml: null,
    };
    onChange(plain, null);
    setFormats(EMPTY_FORMATS);
  }, [draftKey, emitFromDom, onChange, restoreSelection]);

  const handleOpenLinkBar = useCallback(() => {
    saveSelection();
    setShowVarMenu(false);
    setLinkError(null);
    const root = editorRef.current;
    const sel = window.getSelection();
    const existingAnchor = sel ? findAncestorAnchor(sel.anchorNode, root) : null;
    setLinkUrl(existingAnchor?.getAttribute("href") ?? "https://");
    setShowLinkBar((prev) => !prev);
  }, [saveSelection]);

  const handleApplyLink = useCallback(() => {
    const safeUrl = normalizeSafeLinkUrl(linkUrl);
    if (!safeUrl) {
      setLinkError(t("studio.linkInvalidUrl"));
      return;
    }
    setLinkError(null);
    const root = editorRef.current;
    if (!root) return;
    const range = restoreSelection();
    if (!range) return;

    const existingAnchor = findAncestorAnchor(range.commonAncestorContainer, root);
    if (existingAnchor) {
      existingAnchor.setAttribute("href", safeUrl);
    } else if (range.collapsed) {
      const a = document.createElement("a");
      a.setAttribute("href", safeUrl);
      a.textContent = safeUrl;
      range.insertNode(a);
      const after = document.createRange();
      after.setStartAfter(a);
      after.collapse(true);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(after);
    } else {
      const text = range.toString();
      const a = document.createElement("a");
      a.setAttribute("href", safeUrl);
      a.textContent = text;
      range.deleteContents();
      range.insertNode(a);
      const after = document.createRange();
      after.setStartAfter(a);
      after.collapse(true);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(after);
    }

    setShowLinkBar(false);
    emitFromDom();
  }, [emitFromDom, linkUrl, restoreSelection]);

  const handleInsertVariable = useCallback(
    (snippet: string) => {
      setShowVarMenu(false);
      const root = editorRef.current;
      if (!root) return;
      const range = restoreSelection();
      if (!range) return;

      range.deleteContents();
      const textNode = document.createTextNode(snippet);
      range.insertNode(textNode);
      const after = document.createRange();
      after.setStartAfter(textNode);
      after.collapse(true);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(after);
      savedRangeRef.current = after.cloneRange();
      emitFromDom();
    },
    [emitFromDom, restoreSelection],
  );

  const handlePaste = useCallback(
    (e: ClipboardEvent<HTMLDivElement>) => {
      e.preventDefault();
      const htmlData = e.clipboardData.getData("text/html");
      const plainData = e.clipboardData.getData("text/plain");
      const sanitized = htmlData
        ? sanitizeRichHtml(htmlData)
        : plainTextToRichHtml(plainData);

      if (!sanitized) return;

      const root = editorRef.current;
      if (!root) return;
      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0 && root.contains(sel.anchorNode)) {
        const range = sel.getRangeAt(0);
        range.deleteContents();
        const template = document.createElement("template");
        template.innerHTML = sanitized;
        const frag = template.content;
        const lastChild = frag.lastChild;
        range.insertNode(frag);
        if (lastChild) {
          const after = document.createRange();
          after.setStartAfter(lastChild);
          after.collapse(true);
          sel.removeAllRanges();
          sel.addRange(after);
          savedRangeRef.current = after.cloneRange();
        }
      } else {
        root.innerHTML = sanitizeRichHtml(`${root.innerHTML}${sanitized}`);
      }
      emitFromDom();
    },
    [emitFromDom],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (e.ctrlKey || e.metaKey) {
        const key = e.key.toLowerCase();
        if (key === "b") {
          e.preventDefault();
          runExecCommand("bold");
        } else if (key === "i") {
          e.preventDefault();
          runExecCommand("italic");
        } else if (key === "u") {
          e.preventDefault();
          runExecCommand("underline");
        }
      }
    },
    [runExecCommand],
  );

  const isEmpty = !content && !contentHtml;

  return (
    <div className="rich-editor-wrap">
      <div className="unified-toolbar">
        <div className="toolbar-fmt-group">
          <button
            type="button"
            className={`fmt-btn ${formats.bold ? "is-active" : ""}`}
            title={t("tip.fmtBold")}
            aria-label={t("studio.fmtBold")}
            aria-pressed={formats.bold}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => runExecCommand("bold")}
          >
            <strong>B</strong>
          </button>
          <button
            type="button"
            className={`fmt-btn ${formats.italic ? "is-active" : ""}`}
            title={t("tip.fmtItalic")}
            aria-label={t("studio.fmtItalic")}
            aria-pressed={formats.italic}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => runExecCommand("italic")}
          >
            <em>I</em>
          </button>
          <button
            type="button"
            className={`fmt-btn ${formats.underline ? "is-active" : ""}`}
            title={t("tip.fmtUnderline")}
            aria-label={t("studio.fmtUnderline")}
            aria-pressed={formats.underline}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => runExecCommand("underline")}
          >
            <u>U</u>
          </button>

          <span className="toolbar-sep" aria-hidden="true" />

          <button
            type="button"
            className={`fmt-btn ${formats.unorderedList ? "is-active" : ""}`}
            title={t("tip.fmtBulletList")}
            aria-label={t("studio.fmtBulletList")}
            aria-pressed={formats.unorderedList}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => runExecCommand("insertUnorderedList")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <circle cx="3" cy="4.2" r="1.15" fill="currentColor" />
              <circle cx="3" cy="8" r="1.15" fill="currentColor" />
              <circle cx="3" cy="11.8" r="1.15" fill="currentColor" />
              <path
                d="M6.2 4.2H13.5M6.2 8H13.5M6.2 11.8H13.5"
                stroke="currentColor"
                strokeWidth="1.4"
                strokeLinecap="round"
              />
            </svg>
          </button>
          <button
            type="button"
            className={`fmt-btn ${formats.orderedList ? "is-active" : ""}`}
            title={t("tip.fmtOrderedList")}
            aria-label={t("studio.fmtOrderedList")}
            aria-pressed={formats.orderedList}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => runExecCommand("insertOrderedList")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path
                d="M2.2 3.4L3.1 2.8V5.6M2.2 7.5H3.7L2.2 9.2H3.8M2.2 11.3H3.7L2.9 12.2L3.7 13.1H2.2"
                stroke="currentColor"
                strokeWidth="1.1"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d="M6.2 4.2H13.5M6.2 8H13.5M6.2 11.8H13.5"
                stroke="currentColor"
                strokeWidth="1.4"
                strokeLinecap="round"
              />
            </svg>
          </button>

          <span className="toolbar-sep" aria-hidden="true" />

          <button
            type="button"
            className={`fmt-btn ${formats.link || showLinkBar ? "is-active" : ""}`}
            title={t("tip.fmtLink")}
            aria-label={t("studio.fmtLink")}
            aria-pressed={formats.link || showLinkBar}
            onMouseDown={(e) => e.preventDefault()}
            onClick={handleOpenLinkBar}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path
                d="M6.5 9.5L9.5 6.5M5.2 7.2L3.8 8.6C2.7 9.7 2.7 11.4 3.8 12.4C4.9 13.5 6.6 13.5 7.6 12.4L9 11M10.8 8.8L12.2 7.4C13.3 6.3 13.3 4.6 12.2 3.6C11.1 2.5 9.4 2.5 8.4 3.6L7 5"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </button>
          <button
            type="button"
            className="fmt-btn"
            title={t("tip.fmtImage")}
            aria-label={t("studio.fmtImage")}
            onMouseDown={(e) => e.preventDefault()}
            onClick={onRequestInsertImage}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <rect
                x="2"
                y="2.75"
                width="12"
                height="10.5"
                rx="1.75"
                stroke="currentColor"
                strokeWidth="1.4"
              />
              <circle cx="5.6" cy="6.1" r="1.15" fill="currentColor" />
              <path
                d="M2.6 11.4L6.2 8.2L8.8 10.6L10.8 8.9L13.4 11.4"
                stroke="currentColor"
                strokeWidth="1.35"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
          <button
            type="button"
            className="fmt-btn"
            title={t("tip.fmtClearFormat")}
            aria-label={t("studio.fmtClearFormat")}
            onMouseDown={(e) => e.preventDefault()}
            onClick={handleClearFormatting}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path
                d="M3.2 4.2H12.2M7.6 4.2L5.8 12.2M2.8 13.2L13.2 2.8"
                stroke="currentColor"
                strokeWidth="1.4"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </div>

        <div className="toolbar-right-group">
          {contentHtml && (
            <button
              type="button"
              className={`fmt-btn fmt-code-toggle ${showHtmlSource ? "is-active" : ""}`}
              title={t("studio.htmlToggle")}
              onClick={() => setShowHtmlSource((v) => !v)}
            >
              HTML
            </button>
          )}
          <div className="dropdown-wrap">
            <button
              type="button"
              className="link-btn"
              title={t("tip.insertVar")}
              onMouseDown={(e) => {
                e.preventDefault();
                saveSelection();
              }}
              onClick={() => {
                setShowLinkBar(false);
                setShowVarMenu((v) => !v);
              }}
            >
              + {t("studio.insertVar")}
            </button>
            {showVarMenu && (
              <div className="dropdown-menu">
                {variableSnippets.map((vs) => (
                  <button
                    key={vs.label}
                    type="button"
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => handleInsertVariable(vs.snippet)}
                  >
                    {vs.label}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {showLinkBar && (
        <div className="rich-link-bar">
          <input
            type="url"
            className="input rich-link-input"
            autoFocus
            value={linkUrl}
            onChange={(e) => {
              setLinkUrl(e.target.value);
              if (linkError) setLinkError(null);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleApplyLink();
              } else if (e.key === "Escape") {
                e.preventDefault();
                setShowLinkBar(false);
                setLinkError(null);
              }
            }}
            placeholder={t("studio.linkUrlPlaceholder")}
          />
          <button
            type="button"
            className="btn btn-primary btn-xs"
            onClick={handleApplyLink}
          >
            {t("studio.linkApply")}
          </button>
          <button
            type="button"
            className="btn btn-xs"
            onClick={() => {
              setShowLinkBar(false);
              setLinkError(null);
            }}
          >
            {t("studio.linkCancel")}
          </button>
          {linkError && <span className="rich-link-error">{linkError}</span>}
        </div>
      )}

      <div
        ref={editorRef}
        className={`rich-wysiwyg-area ${isEmpty ? "is-empty" : ""}`}
        contentEditable
        suppressContentEditableWarning
        role="textbox"
        aria-multiline="true"
        aria-label={t("studio.tabContent")}
        data-placeholder={placeholder}
        onInput={emitFromDom}
        onBlur={emitFromDom}
        onKeyUp={refreshFormats}
        onMouseUp={refreshFormats}
        onKeyDown={handleKeyDown}
        onPaste={handlePaste}
      />

      {showHtmlSource && contentHtml && (
        <div className="rich-html-strip">
          <span className="muted small">HTML</span>
          <input
            className="inline-html-input"
            value={contentHtml}
            onChange={(e) => {
              const synced = syncEditorHtml(e.target.value);
              onChange(synced.content || content, synced.content_html);
            }}
          />
          <button
            type="button"
            className="mini-icon-btn"
            title={t("studio.htmlClear")}
            onClick={() => {
              const plain = richHtmlToPlainText(contentHtml);
              onChange(plain, null);
              setShowHtmlSource(false);
            }}
          >
            ×
          </button>
        </div>
      )}
    </div>
  );
}
