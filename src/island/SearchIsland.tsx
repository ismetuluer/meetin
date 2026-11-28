import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
  type SetStateAction,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { MascotAnim } from "../mascot/mascotMachine";
import type { PhraseRecord } from "../studio/types";
import { renderPanelTop, type PanelTopProps } from "./IslandPanel";
import type { IslandMode, PillNotice } from "./islandTypes";
import { createPhraseSearchIndex } from "./searchEngine";

interface SearchIslandDeps {
  phrases: PhraseRecord[];
  loadPhrases: () => Promise<void>;
  cancelThrow: () => void;
  lastUserRef: RefObject<number>;
  setMode: Dispatch<SetStateAction<IslandMode>>;
  closeToMascot: () => Promise<void>;
  playAnim: (next: MascotAnim, allowBaaSound?: boolean) => void;
  showNoticePill: (notice: PillNotice, nextAnim: MascotAnim) => void;
}

// Arama adası durumu: sorgu, sonuçlar, klavyeyle gezinme ve yapıştırma
export function useSearchIsland({
  phrases,
  loadPhrases,
  cancelThrow,
  lastUserRef,
  setMode,
  closeToMascot,
  playAnim,
  showNoticePill,
}: SearchIslandDeps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  const openSearch = useCallback(async () => {
    cancelThrow();
    lastUserRef.current = Date.now();
    await loadPhrases();
    setQuery("");
    setSelectedIndex(0);
    setMode("search");
    await invoke("set_mascot_mode", { mode: "search" });
    window.setTimeout(() => searchInputRef.current?.focus(), 40);
  }, [loadPhrases]);

  // Bulanık arama indeksi
  const searchIndex = useMemo(
    () => createPhraseSearchIndex(phrases),
    [phrases],
  );

  const results = useMemo(
    () => searchIndex.search(query, 30),
    [searchIndex, query],
  );

  const selectedPhrase = results[selectedIndex] ?? null;

  const confirmSelection = useCallback(
    async (phrase: PhraseRecord | null) => {
      if (!phrase) {
        playAnim("sad");
        return;
      }
      try {
        await invoke("paste_phrase_from_island", { phraseId: phrase.id });
      } catch (err) {
        showNoticePill(
          { ok: false, text: String(err) },
          "sad",
        );
      }
    },
    [playAnim, showNoticePill],
  );

  function handleSearchKeyDown(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      void closeToMascot();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) =>
        results.length === 0 ? 0 : Math.min(i + 1, results.length - 1),
      );
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      void confirmSelection(selectedPhrase);
    }
  }

  return {
    query,
    setQuery,
    selectedIndex,
    setSelectedIndex,
    searchInputRef,
    openSearch,
    results,
    selectedPhrase,
    confirmSelection,
    handleSearchKeyDown,
  };
}

export type SearchIslandState = ReturnType<typeof useSearchIsland>;

// Arama adası çizimi (bileşen değil: kipler arası DOM yeniden kullanımı korunur)
export function renderSearchIsland(top: PanelTopProps, search: SearchIslandState) {
  const { results, selectedIndex, selectedPhrase } = search;
  return (
    <div className="island-search-panel">
      {renderPanelTop(
        top,
        <>
          <input
            ref={search.searchInputRef}
            className="island-search-input"
            type="text"
            value={search.query}
            onChange={(e) => {
              search.setQuery(e.target.value);
              search.setSelectedIndex(0);
            }}
            onKeyDown={search.handleSearchKeyDown}
            placeholder={t("island.searchPlaceholder")}
            autoFocus
          />
          <button
            type="button"
            className="island-kbd"
            onClick={top.onClose}
          >
            Esc
          </button>
        </>,
      )}

      <div className="island-search-body">
        <div className="island-results">
          {results.length === 0 ? (
            <div className="island-empty">{t("island.noResults")}</div>
          ) : (
            results.map((item, idx) => (
              <button
                key={item.id}
                type="button"
                className={`island-result-item ${idx === selectedIndex ? "is-selected" : ""}`}
                onMouseEnter={() => search.setSelectedIndex(idx)}
                onClick={() => void search.confirmSelection(item)}
              >
                <span className="island-result-abbr">{item.abbr}</span>
                <span className="island-result-snippet">
                  {item.description || item.content}
                </span>
              </button>
            ))
          )}
        </div>

        <div className="island-preview">
          {selectedPhrase ? (
            <>
              <div className="island-preview-title">
                <span className="abbr-chip">{selectedPhrase.abbr}</span>
                <span className="muted small">
                  {selectedPhrase.use_count}×
                </span>
              </div>
              {selectedPhrase.description && (
                <div className="island-preview-desc">
                  {selectedPhrase.description}
                </div>
              )}
              <div className="island-preview-content">
                {selectedPhrase.content}
              </div>
            </>
          ) : (
            <div className="island-empty">{t("island.previewEmpty")}</div>
          )}
        </div>
      </div>

      <div className="island-search-footer">
        <span>{t("island.navHint")}</span>
        <span>{t("island.pasteHint")}</span>
      </div>
    </div>
  );
}
