import { useSyncExternalStore } from "react";
import en from "./en.json";
import tr from "./tr.json";

export type Language = "tr" | "en";
export type TranslationKey = keyof typeof tr;

const catalogs: Record<Language, Record<string, string>> = {
  tr,
  en,
};

let currentLanguage: Language = "tr";
const listeners = new Set<(lang: Language) => void>();

export function normalizeLanguage(raw: string | null | undefined): Language {
  const trimmed = (raw ?? "").trim().toLowerCase();
  if (trimmed === "en" || trimmed.startsWith("en-") || trimmed.startsWith("en_")) {
    return "en";
  }
  return "tr";
}

export function getLanguage(): Language {
  return currentLanguage;
}

export function setLanguage(lang: Language | string): Language {
  const next = normalizeLanguage(lang);
  if (typeof document !== "undefined" && document.documentElement) {
    document.documentElement.lang = next;
  }
  if (next === currentLanguage) {
    return currentLanguage;
  }
  currentLanguage = next;
  for (const cb of listeners) {
    cb(currentLanguage);
  }
  return currentLanguage;
}

export function subscribeLanguage(cb: (lang: Language) => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

export function useLanguage(): Language {
  return useSyncExternalStore(subscribeLanguage, getLanguage, getLanguage);
}

/**
 * Bağımlılıksız reaktif çeviri yardımcısı. Güncel dili okur ve `{param}` yer tutucularını değiştirir.
 */
export function t(
  key: TranslationKey,
  params?: Record<string, string | number>,
): string {
  const activeCatalog = catalogs[currentLanguage] ?? tr;
  const raw = activeCatalog[key] ?? (tr as Record<string, string>)[key];
  if (raw === undefined) {
    return key;
  }
  if (!params) {
    return raw;
  }
  return raw.replace(/\{(\w+)\}/g, (_, name: string) =>
    params[name] !== undefined ? String(params[name]) : `{${name}}`,
  );
}
