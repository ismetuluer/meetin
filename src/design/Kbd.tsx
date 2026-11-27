import type { KeyboardEvent } from "react";
import { t } from "../i18n";
import { keyLabel, parseShortcut, splitShortcutText } from "./shortcutKeys";
import "./kbd.css";

const labels = () => ({ space: t("kbd.space") });

/** "Ctrl+Alt+P" → üç tuş kutucuğu (R3.0e). Ekran okuyucu tuşları sırayla okur. */
export function KbdGroup({ shortcut }: { shortcut: string }) {
  const keys = parseShortcut(shortcut);
  if (!keys.length) return null;
  const l = labels();
  return (
    <span className="meetin-kbd-group">
      {keys.map((key, i) => (
        <kbd key={`${key}-${i}`} className="meetin-kbd">
          {keyLabel(key, l)}
        </kbd>
      ))}
    </span>
  );
}

/** Düz metindeki kısayolları tuş kutucuğuna çevirir (yardım metinleri). */
export function ShortcutText({ text }: { text: string }) {
  return (
    <>
      {splitShortcutText(text).map((part, i) =>
        "keys" in part ? <KbdGroup key={i} shortcut={part.keys} /> : part.text,
      )}
    </>
  );
}

interface ShortcutFieldProps {
  value: string;
  placeholder: string;
  label: string;
  title: string;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
}

/**
 * Kısayol yakalama alanı: odaklanıp tuşlara basılır, değer tuş kutucuklarıyla görünür.
 * Tuş mantığı (yakalama, silme) çağırandadır.
 */
export function ShortcutField({ value, placeholder, label, title, onKeyDown }: ShortcutFieldProps) {
  return (
    <button
      type="button"
      className="meetin-shortcut-field"
      aria-label={value ? `${label}: ${value}` : label}
      title={title}
      onKeyDown={onKeyDown}
    >
      {value ? <KbdGroup shortcut={value} /> : <span className="meetin-shortcut-placeholder">{placeholder}</span>}
    </button>
  );
}
