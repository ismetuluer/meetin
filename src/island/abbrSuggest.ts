/**
 * Seçilen metinden Türkçe (tr-TR) kurallarına uygun baş harf kısaltma önericisi (§4.1, M3.3).
 */

import type { AbbrSuggestMode } from "../studio/types";

export function toTrUpper(text: string): string {
  return text.toLocaleUpperCase("tr-TR");
}

export function normalizeTurkishAbbr(text: string): string {
  return text.trim().toLocaleLowerCase("tr-TR");
}

/**
 * Seçilen kurala göre ifadenin kısaltma önerisini üretir (§4.1, v1.1):
 * - `initials`: Çok kelimeli baş harfler (maks 6), tek kelimede ilk 4 harf
 * - `first_word`: İlk kelime (maks 8 harf)
 * - `last_word`: Son kelime (maks 8 harf)
 * - `tag`: İlk etiket (yoksa baş harflere döner)
 */
export function suggestAbbreviation(
  text: string,
  mode: AbbrSuggestMode = "initials",
  tags: string[] = [],
): string {
  if (mode === "tag") {
    const firstTag = tags
      .map((t) => t.trim().replace(/^#/, ""))
      .find((t) => t.length > 0);
    if (firstTag) {
      return toTrUpper(Array.from(firstTag).slice(0, 8).join(""));
    }
  }

  const words = text
    .trim()
    .split(/\s+/)
    .map((w) => w.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, ""))
    .filter((w) => w.length > 0);

  if (words.length === 0) {
    return "";
  }

  if (mode === "first_word") {
    return toTrUpper(Array.from(words[0]).slice(0, 8).join(""));
  }

  if (mode === "last_word") {
    return toTrUpper(Array.from(words[words.length - 1]).slice(0, 8).join(""));
  }

  if (words.length === 1) {
    const chars = Array.from(words[0]).slice(0, 4).join("");
    return toTrUpper(chars);
  }

  const initials = words
    .slice(0, 6)
    .map((w) => Array.from(w)[0] ?? "")
    .join("");
  return toTrUpper(initials);
}
