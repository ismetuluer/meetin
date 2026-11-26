import tr from "./tr.json";

export type TranslationKey = keyof typeof tr;

const catalog: Record<string, string> = tr;

/**
 * Bağımlılıksız çeviri yardımcısı. `{param}` yer tutucularını değiştirir.
 */
export function t(
  key: TranslationKey,
  params?: Record<string, string | number>,
): string {
  const raw = catalog[key];
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
