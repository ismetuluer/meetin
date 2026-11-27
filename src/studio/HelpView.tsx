import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { KbdGroup, ShortcutText } from "../design/Kbd";
import { t, useLanguage } from "../i18n";
import {
  getHelpSections,
  searchHelpSections,
  type HelpBlock,
  type HelpSection,
  type HelpSectionId,
} from "./helpContent";
import type { AppSettingsDto, ShortcutConfigDto } from "./types";
import "./help.css";

interface HelpViewProps {
  onOpenSettings: () => void;
  onReplayOnboarding: () => void;
  onCreatePhrase: () => void;
}

const DEFAULT_SHORTCUTS: ShortcutConfigDto = {
  pause: "Ctrl+Alt+P",
  search: "Ctrl+Alt+F",
  quick_add: "Ctrl+Alt+E",
  studio: "Ctrl+Alt+S",
  toggle_mascot: "Ctrl+Alt+Space",
};

export function HelpView({
  onOpenSettings,
  onReplayOnboarding,
  onCreatePhrase,
}: HelpViewProps) {
  const lang = useLanguage();
  const [activeId, setActiveId] = useState<HelpSectionId>("quickstart");
  const [query, setQuery] = useState("");
  const [shortcuts, setShortcuts] =
    useState<ShortcutConfigDto>(DEFAULT_SHORTCUTS);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);

  useEffect(() => {
    void invoke<AppSettingsDto>("get_app_settings")
      .then((s) => {
        if (s?.shortcuts) {
          setShortcuts(s.shortcuts);
        }
      })
      .catch(() => {});
  }, []);

  const sections = useMemo(() => getHelpSections(lang), [lang]);
  const isSearching = query.trim().length > 0;
  const matchedSections = useMemo(
    () => searchHelpSections(query, lang),
    [query, lang],
  );

  const visibleSections = useMemo(() => {
    if (isSearching) return matchedSections;
    const found = sections.find((s) => s.id === activeId);
    return found ? [found] : [sections[0]];
  }, [isSearching, matchedSections, sections, activeId]);

  const handleCopySnippet = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopiedCode(code);
      window.setTimeout(() => {
        setCopiedCode((prev) => (prev === code ? null : prev));
      }, 1500);
    } catch {
      // Pano erişimi başarısız olursa yok say
    }
  };

  const renderShortcutRows = (): { keys: string; desc: string }[] => [
    { keys: shortcuts.search, desc: t("help.sc.search") },
    { keys: shortcuts.quick_add, desc: t("help.sc.quickAdd") },
    { keys: shortcuts.studio, desc: t("help.sc.studio") },
    { keys: shortcuts.pause, desc: t("help.sc.pause") },
    { keys: shortcuts.toggle_mascot, desc: t("help.sc.toggleMascot") },
    { keys: "Shift + F3", desc: t("help.sc.shiftF3") },
    { keys: "Ctrl + K", desc: t("help.sc.palette") },
    { keys: "F1", desc: t("help.sc.help") },
  ];

  const renderBlock = (block: HelpBlock, idx: number, openFaq: boolean) => {
    switch (block.type) {
      case "paragraph":
        return (
          <p key={idx} className="help-paragraph">
            <ShortcutText text={block.text} />
          </p>
        );
      case "steps":
        return (
          <div key={idx} className="help-card">
            {block.title && <h4 className="help-card-title">{block.title}</h4>}
            <ol className="help-steps">
              {block.items.map((step, sIdx) => (
                <li key={sIdx}>
                  <ShortcutText text={step} />
                </li>
              ))}
            </ol>
          </div>
        );
      case "bullets":
        return (
          <div key={idx} className="help-card">
            {block.title && <h4 className="help-card-title">{block.title}</h4>}
            <ul className="help-bullets">
              {block.items.map((item, bIdx) => (
                <li key={bIdx}>
                  <strong>{item.label}:</strong> <ShortcutText text={item.text} />
                </li>
              ))}
            </ul>
          </div>
        );
      case "variables":
        return (
          <div key={idx} className="help-card">
            {block.title && <h4 className="help-card-title">{block.title}</h4>}
            <div className="help-var-list">
              {block.items.map((item) => (
                <div key={item.code} className="help-var-row">
                  <div className="help-var-code-col">
                    <code className="help-code-pill">{item.code}</code>
                    <span className="help-var-example muted small">
                      {item.example}
                    </span>
                  </div>
                  <span className="help-var-desc">{item.desc}</span>
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={() => void handleCopySnippet(item.code)}
                  >
                    {copiedCode === item.code
                      ? t("help.copied")
                      : t("help.copySnippet")}
                  </button>
                </div>
              ))}
            </div>
          </div>
        );
      case "table":
        return (
          <div key={idx} className="help-card">
            {block.title && <h4 className="help-card-title">{block.title}</h4>}
            <div className="help-table-wrap">
              <table className="help-table">
                <thead>
                  <tr>
                    {block.headers.map((h) => (
                      <th key={h}>{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {block.rows.map((row, rIdx) => (
                    <tr key={rIdx}>
                      {row.map((cell, cIdx) => (
                        <td key={cIdx}>
                          <ShortcutText text={cell} />
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        );
      case "tip":
        return (
          <div
            key={idx}
            className={`help-callout ${block.tone === "warn" ? "is-warn" : "is-info"}`}
          >
            <ShortcutText text={block.text} />
          </div>
        );
      case "faq":
        return (
          <div key={idx} className="help-faq-list">
            {block.items.map((item, fIdx) => (
              <details
                key={item.q}
                className="help-faq-item"
                open={openFaq || fIdx === 0}
              >
                <summary className="help-faq-q">{item.q}</summary>
                <p className="help-faq-a">
                  <ShortcutText text={item.a} />
                </p>
              </details>
            ))}
          </div>
        );
    }
  };

  const renderSection = (section: HelpSection) => (
    <article key={section.id} className="help-section">
      <div className="help-section-head">
        <h2>{section.title}</h2>
        <p className="muted">{section.summary}</p>
      </div>

      {section.blocks.map((block, idx) =>
        renderBlock(block, idx, isSearching),
      )}

      {section.id === "shortcuts" && (
        <div className="help-card">
          <div className="help-table-wrap">
            <table className="help-table">
              <thead>
                <tr>
                  <th>{t("help.colShortcut")}</th>
                  <th>{t("help.colAction")}</th>
                </tr>
              </thead>
              <tbody>
                {renderShortcutRows().map((row) => (
                  <tr key={row.keys}>
                    <td>
                      <KbdGroup shortcut={row.keys} />
                    </td>
                    <td>{row.desc}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="muted small" style={{ margin: "10px 0 0" }}>
            {t("help.liveShortcutNote")}
          </p>
        </div>
      )}
    </article>
  );

  return (
    <div className="help-shell">
      <aside className="help-sidebar">
        <div className="help-search-box">
          <input
            className="input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("help.searchPlaceholder")}
          />
          {query && (
            <button
              type="button"
              className="help-search-clear"
              title={t("help.clearSearch")}
              onClick={() => setQuery("")}
            >
              ×
            </button>
          )}
        </div>

        <nav className="help-nav" aria-label={t("help.title")}>
          {sections.map((sec) => {
            const isMatched = matchedSections.some((m) => m.id === sec.id);
            const isActive = !isSearching && activeId === sec.id;
            return (
              <button
                key={sec.id}
                type="button"
                className={`help-nav-item ${isActive ? "is-active" : ""} ${
                  isSearching && !isMatched ? "is-dimmed" : ""
                }`}
                onClick={() => {
                  setQuery("");
                  setActiveId(sec.id);
                }}
              >
                {sec.title}
              </button>
            );
          })}
        </nav>

        <div className="help-quick-actions">
          <span className="tiny-label">{t("help.quickActions")}</span>
          <button
            type="button"
            className="btn btn-primary btn-sm"
            onClick={onCreatePhrase}
          >
            + {t("help.newPhrase")}
          </button>
          <button
            type="button"
            className="btn btn-sm"
            onClick={onOpenSettings}
          >
            {t("help.openSettings")}
          </button>
          <button
            type="button"
            className="btn btn-sm"
            onClick={onReplayOnboarding}
          >
            {t("help.replayTour")}
          </button>
        </div>
      </aside>

      <main className="help-content">
        <div className="help-content-inner">
          <div className="help-banner">
            <h1>{t("help.title")}</h1>
            <p className="muted small">{t("help.subtitle")}</p>
          </div>

          {isSearching && (
            <div className="help-search-status">
              {matchedSections.length > 0
                ? t("help.searchResultsCount", { count: matchedSections.length })
                : t("help.searchEmpty")}
            </div>
          )}

          {visibleSections.length > 0 && visibleSections.map(renderSection)}
        </div>
      </main>
    </div>
  );
}
