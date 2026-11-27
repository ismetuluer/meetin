/* Kısayol metni → tuş listesi (R3.0e). Saf fonksiyonlar; görünüm Kbd.tsx'te. */

/** "Ctrl+Alt+P", "Shift + F3" → ["Ctrl", "Alt", "P"]. Sondaki "+" artı tuşudur. */
export function parseShortcut(text: string): string[] {
  const trimmed = text.trim();
  if (!trimmed) return [];
  const plusKey = /\+\s*\+$/.test(trimmed);
  const body = plusKey ? trimmed.replace(/\+\s*\+$/, "") : trimmed;
  const keys = body
    .split("+")
    .map((k) => k.trim())
    .filter(Boolean);
  if (plusKey) keys.push("+");
  return keys;
}

const NAMES: Record<string, string> = {
  control: "Ctrl",
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  super: "Win",
  meta: "Win",
  win: "Win",
  cmdorctrl: "Ctrl",
  enter: "Enter ↵",
  return: "Enter ↵",
  tab: "Tab ⇥",
  escape: "Esc",
  esc: "Esc",
  backspace: "Backspace ⌫",
  delete: "Del",
  arrowup: "↑",
  up: "↑",
  arrowdown: "↓",
  down: "↓",
  arrowleft: "←",
  left: "←",
  arrowright: "→",
  right: "→",
};

/** Tek tuşun Windows'taki görünen adı. Boşluk tuşu dile göre verilir. */
export function keyLabel(key: string, labels: { space: string }): string {
  const lower = key.toLowerCase();
  if (lower === "space" || key === " ") return labels.space;
  if (NAMES[lower]) return NAMES[lower];
  return key.length === 1 ? key.toUpperCase() : key;
}

export type ShortcutTextPart = { text: string } | { keys: string };

const MODIFIER = "(?:Ctrl|Alt|Shift|Win)";
const KEY = "(?:F(?:1[0-2]|[1-9])|Space|Enter|Tab|Esc|[A-Z0-9])";
/**
 * Metin içindeki kısayol: değiştirici(ler) + tuş ya da yalnız F1–F12. Ardından harf gelmemeli
 * ("Ctrl+tıkla", "Ctrl+bırak" gibi Türkçe kullanımlar kısayol sayılmaz).
 */
const SHORTCUT_IN_TEXT = new RegExp(
  `(?<![\\p{L}\\p{N}])(?:${MODIFIER}(?:\\s*\\+\\s*${MODIFIER})*\\s*\\+\\s*${KEY}|F(?:1[0-2]|[1-9]))(?![\\p{L}\\p{N}])`,
  "gu",
);

/** Düz metni kısayol ve yazı parçalarına böler (yardım metinleri için). */
export function splitShortcutText(text: string): ShortcutTextPart[] {
  const parts: ShortcutTextPart[] = [];
  let last = 0;
  for (const m of text.matchAll(SHORTCUT_IN_TEXT)) {
    const start = m.index ?? 0;
    if (start > last) parts.push({ text: text.slice(last, start) });
    parts.push({ keys: m[0] });
    last = start + m[0].length;
  }
  if (last < text.length) parts.push({ text: text.slice(last) });
  return parts;
}
