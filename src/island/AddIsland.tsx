import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { MascotAnim } from "../mascot/mascotMachine";
import type {
  AbbrSuggestMode,
  AppSettingsDto,
  FolderRecord,
  PhraseRecord,
} from "../studio/types";
import { normalizeTurkishAbbr, suggestAbbreviation } from "./abbrSuggest";
import { renderPanelTop, type PanelTopProps } from "./IslandPanel";
import type { IslandMode } from "./islandTypes";

interface AddIslandDeps {
  phrases: PhraseRecord[];
  folders: FolderRecord[];
  setFolders: Dispatch<SetStateAction<FolderRecord[]>>;
  loadPhrases: () => Promise<void>;
  cancelThrow: () => void;
  lastUserRef: RefObject<number>;
  setMode: Dispatch<SetStateAction<IslandMode>>;
  closeToMascot: () => Promise<void>;
  playAnim: (next: MascotAnim, allowBaaSound?: boolean) => void;
}

// Ekleme adası durumu (§4.4)
export function useAddIsland({
  phrases,
  folders,
  setFolders,
  loadPhrases,
  cancelThrow,
  lastUserRef,
  setMode,
  closeToMascot,
  playAnim,
}: AddIslandDeps) {
  const [abbrSuggestMode, setAbbrSuggestMode] = useState<AbbrSuggestMode>("initials");
  const [addAbbr, setAddAbbr] = useState("");
  const [addAbbrManual, setAddAbbrManual] = useState(false);
  const [addContent, setAddContent] = useState("");
  const [addDescription, setAddDescription] = useState("");
  const [addDescriptionManual, setAddDescriptionManual] = useState(false);
  const [addFolderId, setAddFolderId] = useState<number | null>(() => {
    try {
      const saved = window.localStorage.getItem("meetin:last_folder_id");
      return saved ? Number(saved) : null;
    } catch {
      return null;
    }
  });
  const [showInlineNewFolder, setShowInlineNewFolder] = useState(false);
  const [inlineFolderName, setInlineFolderName] = useState("");
  const [addTagsText, setAddTagsText] = useState("");
  const [addError, setAddError] = useState<string | null>(null);

  const addAbbrInputRef = useRef<HTMLInputElement | null>(null);
  const lastUsedFolderIdRef = useRef<number | null>(addFolderId);

  const openAddIsland = useCallback(
    async (initialText: string) => {
      cancelThrow();
      lastUserRef.current = Date.now();
      const [settingsRes] = await Promise.all([
        invoke<AppSettingsDto>("get_app_settings").catch(() => null),
        loadPhrases(),
      ]);
      const effMode = settingsRes?.abbr_suggest_mode ?? abbrSuggestMode;
      if (settingsRes?.abbr_suggest_mode) {
        setAbbrSuggestMode(settingsRes.abbr_suggest_mode);
      }
      const cleaned = initialText.trim();
      const firstLine =
        cleaned
          .split(/\r?\n/)
          .map((l) => l.trim())
          .find(Boolean) ?? "";
      const initialFid = lastUsedFolderIdRef.current;
      const initialFolderName =
        initialFid !== null
          ? folders.find((f) => f.id === initialFid)?.name ?? ""
          : "";
      setAddContent(initialText);
      setAddAbbr(suggestAbbreviation(cleaned, effMode, initialFolderName ? [initialFolderName] : []));
      setAddAbbrManual(false);
      setAddDescription(firstLine.slice(0, 80));
      setAddDescriptionManual(false);
      setAddFolderId(initialFid);
      setShowInlineNewFolder(false);
      setInlineFolderName("");
      setAddTagsText(initialFolderName);
      setAddError(null);
      setMode("add");
      await invoke("set_mascot_mode", { mode: "add" });
      window.setTimeout(() => {
        addAbbrInputRef.current?.focus();
        addAbbrInputRef.current?.select();
      }, 45);
    },
    [abbrSuggestMode, loadPhrases],
  );

  const conflictingPhrase = useMemo(() => {
    const norm = normalizeTurkishAbbr(addAbbr);
    if (!norm) return null;
    return phrases.find((p) => normalizeTurkishAbbr(p.abbr) === norm) ?? null;
  }, [addAbbr, phrases]);

  async function handleFillFromClipboard() {
    try {
      const clip = await invoke<string | null>("read_clipboard_text");
      if (clip) {
        setAddContent(clip);
        if (!addAbbrManual || !addAbbr.trim()) {
          const tags = addTagsText
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean);
          setAddAbbr(suggestAbbreviation(clip, abbrSuggestMode, tags));
        }
        if (!addDescriptionManual || !addDescription.trim()) {
          const line =
            clip
              .split(/\r?\n/)
              .map((l) => l.trim())
              .find(Boolean) ?? "";
          setAddDescription(line.slice(0, 80));
        }
        setAddError(null);
      } else {
        setAddError(t("add.clipboardEmpty"));
      }
    } catch (err) {
      setAddError(String(err));
    }
  }

  async function handleCreateInlineFolder() {
    const trimmed = inlineFolderName.trim();
    if (!trimmed) return;
    try {
      const created = await invoke<FolderRecord>("create_folder", {
        name: trimmed,
        parentId: null,
        color: null,
      });
      setFolders((prev) => [...prev, created]);
      setAddFolderId(created.id);
      lastUsedFolderIdRef.current = created.id;
      setAddTagsText((prev) => {
        const list = prev.split(",").map((s) => s.trim()).filter(Boolean);
        return list.includes(created.name) ? prev : [...list, created.name].join(", ");
      });
      try {
        window.localStorage.setItem("meetin:last_folder_id", String(created.id));
      } catch {
        // yok say
      }
      setInlineFolderName("");
      setShowInlineNewFolder(false);
    } catch (err) {
      setAddError(String(err));
    }
  }

  async function handleSaveAdd() {
    const abbr = addAbbr.trim();
    if (!abbr || !addContent.trim()) {
      setAddError(t("error.requiredFields"));
      return;
    }
    const tags = addTagsText
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    try {
      await invoke<PhraseRecord>("create_phrase", {
        input: {
          abbr,
          content: addContent,
          content_html: null,
          description: addDescription.trim(),
          folder_id: addFolderId,
          folder_ids: addFolderId !== null ? [addFolderId] : [],
          case_mode: "keep_original",
          shortcut: null,
          tags,
        },
      });
      lastUsedFolderIdRef.current = addFolderId;
      try {
        if (addFolderId !== null) {
          window.localStorage.setItem("meetin:last_folder_id", String(addFolderId));
        } else {
          window.localStorage.removeItem("meetin:last_folder_id");
        }
      } catch {
        // yok say
      }
      await loadPhrases();
      await closeToMascot();
      playAnim("thumbsup");
    } catch (err) {
      setAddError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleDetailedEdit() {
    await invoke("open_studio_with_draft", {
      abbr: addAbbr.trim(),
      content: addContent,
      description: addDescription.trim(),
    });
    await closeToMascot();
  }

  return {
    folders,
    abbrSuggestMode,
    setAbbrSuggestMode,
    addAbbr,
    setAddAbbr,
    addAbbrManual,
    setAddAbbrManual,
    addContent,
    setAddContent,
    addDescription,
    setAddDescription,
    addDescriptionManual,
    setAddDescriptionManual,
    addFolderId,
    setAddFolderId,
    showInlineNewFolder,
    setShowInlineNewFolder,
    inlineFolderName,
    setInlineFolderName,
    addTagsText,
    setAddTagsText,
    addError,
    addAbbrInputRef,
    lastUsedFolderIdRef,
    openAddIsland,
    conflictingPhrase,
    handleFillFromClipboard,
    handleCreateInlineFolder,
    handleSaveAdd,
    handleDetailedEdit,
  };
}

export type AddIslandState = ReturnType<typeof useAddIsland>;

// Ekleme adası çizimi (bileşen değil: kipler arası DOM yeniden kullanımı korunur)
export function renderAddIsland(top: PanelTopProps, add: AddIslandState) {
  const {
    folders,
    abbrSuggestMode,
    addAbbrManual,
    addContent,
    addDescriptionManual,
    addFolderId,
    showInlineNewFolder,
    addTagsText,
    conflictingPhrase,
  } = add;
  return (
    <div className="island-search-panel">
      {renderPanelTop(
        top,
        <>
          <span className="island-add-title">{t("add.title")}</span>
          <button
            type="button"
            className="btn island-mini-btn"
            onClick={() => void add.handleFillFromClipboard()}
          >
            {t("add.fromClipboard")}
          </button>
          <button
            type="button"
            className="island-kbd"
            onClick={top.onClose}
          >
            Esc
          </button>
        </>,
      )}

      <form
        className="island-add-body"
        onSubmit={(e) => {
          e.preventDefault();
          void add.handleSaveAdd();
        }}
      >
        <div className="island-add-row">
          <label className="island-field">
            <span>{t("form.abbrLabel")}</span>
            <input
              ref={add.addAbbrInputRef}
              className="input"
              value={add.addAbbr}
              onChange={(e) => {
                add.setAddAbbr(e.target.value);
                add.setAddAbbrManual(true);
              }}
              placeholder={t("form.abbrPlaceholder")}
              autoFocus
            />
          </label>
          <label className="island-field">
            <span>{t("add.folderLabel")}</span>
            <select
              className="select"
              value={showInlineNewFolder ? "__new__" : (addFolderId ?? "")}
              onChange={(e) => {
                const v = e.target.value;
                if (v === "__new__") {
                  add.setShowInlineNewFolder(true);
                  return;
                }
                add.setShowInlineNewFolder(false);
                const fid = v === "" ? null : Number(v);
                add.setAddFolderId(fid);
                add.lastUsedFolderIdRef.current = fid;
                if (fid !== null) {
                  const fname = folders.find((f) => f.id === fid)?.name;
                  if (fname) {
                    add.setAddTagsText((prev) => {
                      const list = prev.split(",").map((s) => s.trim()).filter(Boolean);
                      return list.includes(fname) ? prev : [...list, fname].join(", ");
                    });
                  }
                }
              }}
            >
              <option value="">{t("add.noFolder")}</option>
              {folders.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name}
                </option>
              ))}
              <option value="__new__">{t("add.newFolderOption")}</option>
            </select>
          </label>
        </div>

        {showInlineNewFolder && (
          <div className="island-inline-new-folder">
            <input
              className="input"
              autoFocus
              value={add.inlineFolderName}
              onChange={(e) => add.setInlineFolderName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void add.handleCreateInlineFolder();
                } else if (e.key === "Escape") {
                  e.stopPropagation();
                  add.setShowInlineNewFolder(false);
                }
              }}
              placeholder={t("add.newFolderPlaceholder")}
            />
            <button
              type="button"
              className="btn btn-primary island-mini-btn"
              onClick={() => void add.handleCreateInlineFolder()}
            >
              {t("add.createFolderBtn")}
            </button>
            <button
              type="button"
              className="btn island-mini-btn"
              onClick={() => add.setShowInlineNewFolder(false)}
            >
              ×
            </button>
          </div>
        )}

        {conflictingPhrase && (
          <div className="badge badge-warn">
            {t("add.conflictWarn", {
              abbr: conflictingPhrase.abbr,
              preview: conflictingPhrase.content.slice(0, 32),
            })}
          </div>
        )}

        <label className="island-field">
          <span>{t("add.contentPreview")}</span>
          <textarea
            className="textarea"
            rows={3}
            value={addContent}
            onChange={(e) => {
              const nextContent = e.target.value;
              add.setAddContent(nextContent);
              if (!addDescriptionManual) {
                const firstLine =
                  nextContent
                    .split(/\r?\n/)
                    .map((l) => l.trim())
                    .find(Boolean) ?? "";
                add.setAddDescription(firstLine.slice(0, 80));
              }
              if (!addAbbrManual) {
                const tags = addTagsText
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean);
                add.setAddAbbr(suggestAbbreviation(nextContent, abbrSuggestMode, tags));
              }
            }}
            placeholder={t("form.contentPlaceholder")}
          />
        </label>

        <div className="island-add-row">
          <label className="island-field">
            <span>{t("form.descriptionLabel")}</span>
            <input
              className="input"
              value={add.addDescription}
              onChange={(e) => {
                add.setAddDescription(e.target.value);
                add.setAddDescriptionManual(e.target.value.trim().length > 0);
              }}
              placeholder={t("form.descriptionPlaceholder")}
            />
          </label>
          <label className="island-field">
            <span>{t("form.tagsLabel")}</span>
            <input
              className="input"
              value={addTagsText}
              onChange={(e) => {
                const nextTagsText = e.target.value;
                add.setAddTagsText(nextTagsText);
                if (!addAbbrManual && abbrSuggestMode === "tag") {
                  const tags = nextTagsText
                    .split(",")
                    .map((s) => s.trim())
                    .filter(Boolean);
                  add.setAddAbbr(suggestAbbreviation(addContent, "tag", tags));
                }
              }}
              placeholder={t("form.tagsPlaceholder")}
            />
          </label>
        </div>

        {add.addError && <div className="error-banner">{add.addError}</div>}

        <div className="island-add-actions">
          <button
            type="button"
            className="btn island-mini-btn"
            onClick={() => void add.handleDetailedEdit()}
          >
            {t("add.detailedEdit")}
          </button>
          <div className="island-add-actions-right">
            <button
              type="button"
              className="btn island-mini-btn"
              onClick={top.onClose}
            >
              {t("form.cancel")}
            </button>
            <button type="submit" className="btn btn-primary island-mini-btn">
              {t("form.save")}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
