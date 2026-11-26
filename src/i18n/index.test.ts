import { afterEach, describe, expect, it, vi } from "vitest";
import en from "./en.json";
import {
  getLanguage,
  normalizeLanguage,
  setLanguage,
  subscribeLanguage,
  t,
  type TranslationKey,
} from "./index";
import tr from "./tr.json";

function extractPlaceholders(text: string): string[] {
  const matches = text.match(/\{(\w+)\}/g) ?? [];
  return [...new Set(matches)].sort();
}

describe("i18n catalog and live language switching", () => {
  afterEach(() => {
    setLanguage("tr");
  });

  it("has exact 1-to-1 key parity between tr.json and en.json", () => {
    const trKeys = Object.keys(tr).sort();
    const enKeys = Object.keys(en).sort();

    const missingInEn = trKeys.filter((k) => !(k in en));
    const extraInEn = enKeys.filter((k) => !(k in tr));

    expect(missingInEn).toEqual([]);
    expect(extraInEn).toEqual([]);
    expect(enKeys.length).toBe(trKeys.length);
  });

  it("contains zero empty or whitespace-only strings in both tr.json and en.json", () => {
    const emptyTr = Object.entries(tr).filter(([, v]) => v.trim().length === 0);
    const emptyEn = Object.entries(en).filter(([, v]) => v.trim().length === 0);

    expect(emptyTr).toEqual([]);
    expect(emptyEn).toEqual([]);
  });

  it("preserves identical template placeholders across tr.json and en.json", () => {
    const trMap = tr as Record<string, string>;
    const enMap = en as Record<string, string>;
    for (const key of Object.keys(trMap)) {
      expect(extractPlaceholders(enMap[key]), `placeholder mismatch for ${key}`).toEqual(
        extractPlaceholders(trMap[key]),
      );
    }
  });

  it("returns translated strings for all keys in both Turkish and English", () => {
    setLanguage("tr");
    for (const key of Object.keys(tr) as TranslationKey[]) {
      const value = t(key);
      expect(value).toBeTruthy();
      expect(value).not.toBe(key);
    }

    setLanguage("en");
    for (const key of Object.keys(en) as TranslationKey[]) {
      const value = t(key);
      expect(value).toBeTruthy();
      expect(value).not.toBe(key);
    }
  });

  it("switches live between tr and en and notifies subscribers", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeLanguage(listener);

    expect(getLanguage()).toBe("tr");
    expect(t("status.phraseCount", { count: 42 })).toBe("42 etkin ifade");
    expect(t("error.generic", { message: "test" })).toBe("Bir hata oluştu: test");

    setLanguage("en");
    expect(getLanguage()).toBe("en");
    expect(t("status.phraseCount", { count: 42 })).toBe("42 active phrases");
    expect(t("error.generic", { message: "test" })).toBe("An error occurred: test");
    expect(t("settings.nav.language")).toBe("Language");
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenLastCalledWith("en");

    setLanguage("tr");
    expect(getLanguage()).toBe("tr");
    expect(t("settings.nav.language")).toBe("Dil");
    expect(listener).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenLastCalledWith("tr");

    unsubscribe();
    setLanguage("en");
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("normalizes locale variants safely", () => {
    expect(normalizeLanguage("tr")).toBe("tr");
    expect(normalizeLanguage("tr-TR")).toBe("tr");
    expect(normalizeLanguage("en")).toBe("en");
    expect(normalizeLanguage("en-US")).toBe("en");
    expect(normalizeLanguage("EN_GB")).toBe("en");
    expect(normalizeLanguage("")).toBe("tr");
    expect(normalizeLanguage(null)).toBe("tr");
    expect(normalizeLanguage("fr-FR")).toBe("tr");
  });
});
