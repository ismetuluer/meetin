import { useEffect, useMemo, useState } from "react";
import { KbdGroup } from "../design/Kbd";
import { t } from "../i18n";
import type { PhraseRecord } from "./types";

export interface PaletteCommand {
  id: string;
  label: string;
  hint?: string;
  onRun: () => void;
}

interface CommandPaletteProps {
  phrases: PhraseRecord[];
  commands: PaletteCommand[];
  onSelectPhrase: (phrase: PhraseRecord) => void;
  onClose: () => void;
}

export function CommandPalette({
  phrases,
  commands,
  onSelectPhrase,
  onClose,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);

  const items = useMemo(() => {
    const q = query.trim().toLocaleLowerCase("tr-TR");
    const matchedCmds = commands
      .filter((c) => !q || c.label.toLocaleLowerCase("tr-TR").includes(q))
      .map((c) => ({ type: "cmd" as const, cmd: c }));

    const matchedPhrases = phrases
      .filter(
        (p) =>
          !p.deleted_at &&
          (!q ||
            p.abbr.toLocaleLowerCase("tr-TR").includes(q) ||
            p.content.toLocaleLowerCase("tr-TR").includes(q) ||
            p.description.toLocaleLowerCase("tr-TR").includes(q)),
      )
      .slice(0, 12)
      .map((p) => ({ type: "phrase" as const, phrase: p }));

    return [...matchedCmds, ...matchedPhrases];
  }, [query, commands, phrases]);

  useEffect(() => {
    setActiveIdx(0);
  }, [query]);

  const executeItem = (idx: number) => {
    const entry = items[idx];
    if (!entry) return;
    onClose();
    if (entry.type === "cmd") {
      entry.cmd.onRun();
    } else {
      onSelectPhrase(entry.phrase);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="palette-card"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
          } else if (e.key === "ArrowDown") {
            e.preventDefault();
            setActiveIdx((i) => Math.min(i + 1, Math.max(0, items.length - 1)));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setActiveIdx((i) => Math.max(i - 1, 0));
          } else if (e.key === "Enter") {
            e.preventDefault();
            executeItem(activeIdx);
          }
        }}
      >
        <input
          className="palette-input"
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("palette.placeholder")}
        />
        <div className="palette-list">
          {items.length === 0 ? (
            <div className="muted small" style={{ padding: "12px" }}>
              {t("palette.empty")}
            </div>
          ) : (
            items.map((item, idx) => {
              const isSelected = idx === activeIdx;
              if (item.type === "cmd") {
                return (
                  <button
                    key={item.cmd.id}
                    type="button"
                    className={`palette-item ${isSelected ? "is-active" : ""}`}
                    onClick={() => executeItem(idx)}
                  >
                    <span>{item.cmd.label}</span>
                    {item.cmd.hint && (
                      <KbdGroup shortcut={item.cmd.hint} />
                    )}
                  </button>
                );
              }
              return (
                <button
                  key={`p-${item.phrase.id}`}
                  type="button"
                  className={`palette-item ${isSelected ? "is-active" : ""}`}
                  onClick={() => executeItem(idx)}
                >
                  <span className="palette-phrase-row">
                    <span className="abbr-chip">{item.phrase.abbr}</span>
                    <span className="palette-preview">{item.phrase.content}</span>
                  </span>
                  <span className="muted small">{item.phrase.use_count}</span>
                </button>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
