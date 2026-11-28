/**
 * İfade kopyası için çakışmasız kısaltma (R3.1a). Karşılaştırma Rust'taki
 * `normalize_abbr` ile aynıdır: tr-TR küçük harf, ayar açıksa aksanlar katlanır.
 */
const ACCENT_FOLD: Record<string, string> = {
  ı: "i",
  î: "i",
  ç: "c",
  ğ: "g",
  ö: "o",
  ş: "s",
  ü: "u",
  û: "u",
  â: "a",
};

export function normalizeAbbr(abbr: string, ignoreAccents: boolean): string {
  const lowered = abbr.toLocaleLowerCase("tr-TR");
  return ignoreAccents
    ? lowered.replace(/[ıîçğöşüûâ]/g, (ch) => ACCENT_FOLD[ch] ?? ch)
    : lowered;
}

/** `mipy` → `mipy2`, varsa `mipy3`…; sondaki sayı artırılır (`ad2` → `ad3`). */
export function uniqueCopyAbbr(
  abbr: string,
  existing: string[],
  ignoreAccents: boolean,
): string {
  const taken = new Set(existing.map((a) => normalizeAbbr(a, ignoreAccents)));
  const trimmed = abbr.trim();
  if (!trimmed) return "";
  const m = /^(.*?)(\d+)$/.exec(trimmed);
  const stem = m && m[1] ? m[1] : trimmed;
  let n = m && m[1] ? Number(m[2]) + 1 : 2;
  for (let guard = 0; guard < 10000; guard++, n++) {
    const candidate = `${stem}${n}`;
    if (!taken.has(normalizeAbbr(candidate, ignoreAccents))) return candidate;
  }
  return "";
}
