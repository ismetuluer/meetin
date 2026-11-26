import type { KeyboardEvent } from "react";

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph"]);

/**
 * Klavye olayından Tauri/Windows uyumlu kısayol metni üretir (örn. `Ctrl+Alt+P`).
 */
export function captureShortcutCombo(e: KeyboardEvent<HTMLInputElement>): string | null {
  if (MODIFIER_KEYS.has(e.key)) {
    return null;
  }
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  if (mods.length === 0) {
    return null;
  }

  let keyName = "";
  if (e.code === "Space" || e.key === " ") {
    keyName = "Space";
  } else if (/^Key([A-Z])$/.test(e.code)) {
    keyName = e.code.slice(3);
  } else if (/^Digit([0-9])$/.test(e.code)) {
    keyName = e.code.slice(5);
  } else if (/^F([1-9]|1[0-2])$/.test(e.key)) {
    keyName = e.key;
  } else if (e.key.length === 1 && /[a-zA-Z0-9]/.test(e.key)) {
    keyName = e.key.toUpperCase();
  } else {
    return null;
  }

  return `${mods.join("+")}+${keyName}`;
}
