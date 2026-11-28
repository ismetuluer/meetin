/**
 * Meetin OKLCH Palet Üretici (§11.2, §11.3, §11.4).
 * Sıfır dış bağımlılıkla algısal olarak eşit renk ölçekleri ve WCAG >= 4,5:1
 * karşıtlık güvencesi üretir.
 */

export type ThemeMode = "system" | "light" | "dark";
export type PaletteSource = "system" | "custom";
export type IslandSurface = "match_theme" | "always_dark";
export type FontSizeOption = "sm" | "md" | "lg" | "xl";

export const FONT_SIZE_PX: Record<FontSizeOption, number> = {
  sm: 12,
  md: 14,
  lg: 16,
  xl: 18,
};

export type ThemeRoleKey =
  | "--vurgu"
  | "--vurgu-hover"
  | "--vurgu-basili"
  | "--vurgu-ustu"
  | "--vurgu-zayif"
  | "--zemin"
  | "--yuzey"
  | "--yuzey-2"
  | "--cizgi"
  | "--metin"
  | "--metin-ikincil"
  | "--metin-pasif"
  | "--basari"
  | "--uyari"
  | "--hata";

export type ThemePalette = Record<ThemeRoleKey, string>;

export interface ThemeConfig {
  mode: ThemeMode;
  source: PaletteSource;
  accentHex: string;
  islandSurface: IslandSurface;
  acrylicOpacity: number;
  fontSize?: FontSizeOption;
  overrides: Partial<Record<ThemeRoleKey, string>>;
}

export const DEFAULT_ACCENT_HEX = "#0078D4";

export const DEFAULT_THEME_CONFIG: ThemeConfig = {
  mode: "system",
  source: "system",
  accentHex: "#7C4DFF",
  islandSurface: "match_theme",
  acrylicOpacity: 0.92,
  fontSize: "md",
  overrides: {},
};

export interface PresetAccent {
  name: string;
  hex: string;
}

export const PRESET_ACCENTS: PresetAccent[] = [
  { name: "Mavi", hex: "#0078D4" },
  { name: "Mor", hex: "#7C4DFF" },
  { name: "Turkuaz", hex: "#0E9488" },
  { name: "Orman", hex: "#1E8E5A" },
  { name: "Kehribar", hex: "#D97706" },
  { name: "Gül", hex: "#D6336C" },
  { name: "Grafit", hex: "#525B6B" },
];

export const SCALE_STEPS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950] as const;
export type ScaleStep = (typeof SCALE_STEPS)[number];

export interface ThemeRoleMeta {
  key: ThemeRoleKey;
  contrastAgainst?: ThemeRoleKey;
}

export const THEME_ROLES: ThemeRoleMeta[] = [
  { key: "--vurgu", contrastAgainst: "--yuzey" },
  { key: "--vurgu-ustu", contrastAgainst: "--vurgu" },
  { key: "--vurgu-zayif", contrastAgainst: "--metin" },
  { key: "--zemin", contrastAgainst: "--metin" },
  { key: "--yuzey", contrastAgainst: "--metin" },
  { key: "--yuzey-2", contrastAgainst: "--metin" },
  { key: "--cizgi" },
  { key: "--metin", contrastAgainst: "--yuzey" },
  { key: "--metin-ikincil", contrastAgainst: "--yuzey" },
  { key: "--metin-pasif" },
  { key: "--vurgu-hover" },
  { key: "--vurgu-basili" },
  { key: "--basari", contrastAgainst: "--yuzey" },
  { key: "--uyari", contrastAgainst: "--yuzey" },
  { key: "--hata", contrastAgainst: "--yuzey" },
];

export interface OklchColor {
  l: number;
  c: number;
  h: number;
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

export function normalizeHex(input: string, fallback = DEFAULT_ACCENT_HEX): string {
  const clean = input.trim().replace(/^#/, "");
  if (/^[0-9a-fA-F]{6}$/.test(clean)) {
    return `#${clean.toUpperCase()}`;
  }
  if (/^[0-9a-fA-F]{3}$/.test(clean)) {
    const exp = clean
      .split("")
      .map((ch) => ch + ch)
      .join("");
    return `#${exp.toUpperCase()}`;
  }
  return fallback;
}

export function hexToRgb(hex: string): [number, number, number] {
  const norm = normalizeHex(hex);
  const n = parseInt(norm.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

export function rgbToHex(r: number, g: number, b: number): string {
  const toByte = (v: number) =>
    Math.round(clamp01(v) * 255)
      .toString(16)
      .padStart(2, "0")
      .toUpperCase();
  return `#${toByte(r)}${toByte(g)}${toByte(b)}`;
}

function srgbToLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function linearToSrgb(c: number): number {
  return c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
}

export function rgbToOklch(r: number, g: number, b: number): OklchColor {
  const lr = srgbToLinear(r);
  const lg = srgbToLinear(g);
  const lb = srgbToLinear(b);

  const l_ = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m_ = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s_ = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);

  const L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_;
  const a = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_;
  const bOk = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_;

  const C = Math.hypot(a, bOk);
  let H = (Math.atan2(bOk, a) * 180) / Math.PI;
  if (H < 0) H += 360;

  return { l: L, c: C, h: Number.isFinite(H) ? H : 0 };
}

function oklchToLinearRgbRaw(l: number, c: number, hDeg: number): [number, number, number] {
  const rad = (hDeg * Math.PI) / 180;
  const a = c * Math.cos(rad);
  const b = c * Math.sin(rad);

  const l_ = l + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = l - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = l - 0.0894841775 * a - 1.291485548 * b;

  const l3 = l_ * l_ * l_;
  const m3 = m_ * m_ * m_;
  const s3 = s_ * s_ * s_;

  return [
    4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
    -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
    -0.0041960863 * l3 - 0.7034186147 * m3 + 1.707614701 * s3,
  ];
}

function inGamut(lr: number, lg: number, lb: number): boolean {
  const eps = 1e-4;
  return (
    lr >= -eps &&
    lr <= 1 + eps &&
    lg >= -eps &&
    lg <= 1 + eps &&
    lb >= -eps &&
    lb <= 1 + eps
  );
}

/** Gamut dışına taşan doygunluğu ikili aramayla sRGB içinde kırpar (§11.3). */
export function oklchToRgb(l: number, c: number, h: number): [number, number, number] {
  const lc = clamp01(l);
  const cc = Math.max(0, c);
  const raw = oklchToLinearRgbRaw(lc, cc, h);
  if (inGamut(raw[0], raw[1], raw[2])) {
    return [
      clamp01(linearToSrgb(clamp01(raw[0]))),
      clamp01(linearToSrgb(clamp01(raw[1]))),
      clamp01(linearToSrgb(clamp01(raw[2]))),
    ];
  }

  let lo = 0;
  let hi = cc;
  for (let i = 0; i < 16; i++) {
    const mid = (lo + hi) * 0.5;
    const probe = oklchToLinearRgbRaw(lc, mid, h);
    if (inGamut(probe[0], probe[1], probe[2])) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  const clipped = oklchToLinearRgbRaw(lc, lo, h);
  return [
    clamp01(linearToSrgb(clamp01(clipped[0]))),
    clamp01(linearToSrgb(clamp01(clipped[1]))),
    clamp01(linearToSrgb(clamp01(clipped[2]))),
  ];
}

export function oklchToHex(l: number, c: number, h: number): string {
  const [r, g, b] = oklchToRgb(l, c, h);
  return rgbToHex(r, g, b);
}

export function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  return 0.2126 * srgbToLinear(r) + 0.7152 * srgbToLinear(g) + 0.0722 * srgbToLinear(b);
}

export function contrastRatio(fgHex: string, bgHex: string): number {
  const l1 = relativeLuminance(fgHex);
  const l2 = relativeLuminance(bgHex);
  const hi = Math.max(l1, l2);
  const lo = Math.min(l1, l2);
  return (hi + 0.05) / (lo + 0.05);
}

export function hueFromHex(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  const oklch = rgbToOklch(r, g, b);
  return Math.round(oklch.h);
}

export function hexWithHue(hex: string, newHue: number): string {
  const [r, g, b] = hexToRgb(hex);
  const oklch = rgbToOklch(r, g, b);
  const c = oklch.c < 0.06 ? 0.17 : oklch.c;
  const l = oklch.l < 0.35 || oklch.l > 0.78 ? 0.56 : oklch.l;
  return oklchToHex(l, c, ((newHue % 360) + 360) % 360);
}

/** 50..950 11 basamaklı algısal olarak eşit vurgu ölçeği (§11.3). */
export function generateAccentScale(accentHex: string): Record<ScaleStep, string> {
  const [r, g, b] = hexToRgb(accentHex);
  const base = rgbToOklch(r, g, b);
  const scaleTargets: Record<ScaleStep, { l: number; cMul: number }> = {
    50: { l: 0.97, cMul: 0.18 },
    100: { l: 0.93, cMul: 0.32 },
    200: { l: 0.86, cMul: 0.52 },
    300: { l: 0.78, cMul: 0.74 },
    400: { l: 0.69, cMul: 0.92 },
    500: { l: 0.59, cMul: 1.0 },
    600: { l: 0.51, cMul: 1.0 },
    700: { l: 0.43, cMul: 0.94 },
    800: { l: 0.35, cMul: 0.84 },
    900: { l: 0.29, cMul: 0.72 },
    950: { l: 0.25, cMul: 0.6 },
  };
  const out = {} as Record<ScaleStep, string>;
  for (const step of SCALE_STEPS) {
    const { l, cMul } = scaleTargets[step];
    out[step] = oklchToHex(l, base.c * cMul, base.h);
  }
  return out;
}

function ensureContrast(
  initialL: number,
  c: number,
  h: number,
  against: string[],
  preferDarker: boolean,
  minRatio = 4.6,
): string {
  let l = clamp01(initialL);
  let best = oklchToHex(l, c, h);
  let bestMin = Math.min(...against.map((bg) => contrastRatio(best, bg)));
  if (bestMin >= minRatio) {
    return best;
  }

  for (let i = 0; i < 95; i++) {
    l = clamp01(l + (preferDarker ? -0.012 : 0.012));
    const cand = oklchToHex(l, c, h);
    const minC = Math.min(...against.map((bg) => contrastRatio(cand, bg)));
    if (minC > bestMin) {
      bestMin = minC;
      best = cand;
    }
    if (minC >= minRatio) {
      return cand;
    }
    if ((preferDarker && l <= 0.01) || (!preferDarker && l >= 0.995)) {
      break;
    }
  }
  return best;
}

/** Renk üzerinde okunacak metin rengi (açık ya da koyu), karşıtlığa göre. */
export function pickOnAccentText(accentHex: string): string {
  const lightText = "#FFFFFF";
  const darkText = "#14111C";
  return contrastRatio(lightText, accentHex) >= contrastRatio(darkText, accentHex)
    ? lightText
    : darkText;
}

/** Açık veya koyu tema için tam ve WCAG >= 4,5:1 güvenceli palet üretir (§11.3). */
export function generatePalette(accentHex: string, dark: boolean): ThemePalette {
  const [r, g, b] = hexToRgb(accentHex);
  const base = rgbToOklch(r, g, b);
  const h = base.h;
  const neutralC = Math.min(0.012, base.c * 0.08 + 0.004);
  const accentC = Math.max(0.02, base.c);

  const zemin = dark
    ? oklchToHex(0.185, neutralC, h)
    : oklchToHex(0.972, neutralC, h);
  const yuzey = dark
    ? oklchToHex(0.228, neutralC, h)
    : oklchToHex(0.996, neutralC * 0.4, h);
  const yuzey2 = dark
    ? oklchToHex(0.268, neutralC, h)
    : oklchToHex(0.946, neutralC, h);
  const cizgi = dark
    ? oklchToHex(0.328, neutralC * 1.1, h)
    : oklchToHex(0.902, neutralC * 1.1, h);

  const surfaceBg = [zemin, yuzey, yuzey2];
  const metin = ensureContrast(dark ? 0.955 : 0.21, neutralC, h, surfaceBg, !dark, 10.0);
  const metinIkincil = ensureContrast(
    dark ? 0.77 : 0.455,
    neutralC,
    h,
    surfaceBg,
    !dark,
    4.65,
  );
  const metinPasif = oklchToHex(dark ? 0.6 : 0.58, neutralC, h);

  const vurguZayif = dark
    ? oklchToHex(0.285, Math.min(0.055, accentC * 0.35), h)
    : oklchToHex(0.942, Math.min(0.036, accentC * 0.25), h);

  // Vurgu rengi hem yuzey/vurguZayif hem de vurguUstu metniyle >= 4,5:1 vermeli
  let vurguL = dark ? Math.max(0.7, base.l) : Math.min(0.55, base.l);
  let vurgu = oklchToHex(vurguL, accentC, h);
  let vurguUstu = pickOnAccentText(vurgu);

  for (let i = 0; i < 65; i++) {
    const okBg =
      contrastRatio(vurgu, yuzey) >= 4.6 &&
      contrastRatio(vurgu, vurguZayif) >= 4.6 &&
      contrastRatio(vurgu, zemin) >= 4.6;
    const okText = contrastRatio(vurguUstu, vurgu) >= 4.6;
    if (okBg && okText) break;

    vurguL = clamp01(vurguL + (dark ? 0.012 : -0.012));
    vurgu = oklchToHex(vurguL, accentC, h);
    vurguUstu = dark ? "#14111C" : "#FFFFFF";
    if (contrastRatio(pickOnAccentText(vurgu), vurgu) > contrastRatio(vurguUstu, vurgu)) {
      vurguUstu = pickOnAccentText(vurgu);
    }
  }

  const vurguHover = oklchToHex(clamp01(vurguL + (dark ? 0.05 : -0.05)), accentC, h);
  const vurguBasili = oklchToHex(clamp01(vurguL + (dark ? -0.05 : -0.1)), accentC, h);

  const basari = ensureContrast(dark ? 0.76 : 0.52, 0.155, 146, [yuzey, yuzey2], !dark, 4.65);
  const uyari = ensureContrast(dark ? 0.80 : 0.55, 0.155, 64, [yuzey, yuzey2], !dark, 4.65);
  const hata = ensureContrast(dark ? 0.74 : 0.52, 0.185, 25, [yuzey, yuzey2], !dark, 4.65);

  return {
    "--vurgu": vurgu,
    "--vurgu-hover": vurguHover,
    "--vurgu-basili": vurguBasili,
    "--vurgu-ustu": vurguUstu,
    "--vurgu-zayif": vurguZayif,
    "--zemin": zemin,
    "--yuzey": yuzey,
    "--yuzey-2": yuzey2,
    "--cizgi": cizgi,
    "--metin": metin,
    "--metin-ikincil": metinIkincil,
    "--metin-pasif": metinPasif,
    "--basari": basari,
    "--uyari": uyari,
    "--hata": hata,
  };
}

export interface ResolvedTheme {
  resolvedDark: boolean;
  effectiveAccentHex: string;
  scale: Record<ScaleStep, string>;
  generated: ThemePalette;
  resolved: ThemePalette;
  adaZemin: string;
  adaMetin: string;
}

export function resolveThemeTokens(
  config: ThemeConfig,
  systemDark: boolean,
  systemAccentHex: string,
): ResolvedTheme {
  const resolvedDark =
    config.mode === "dark" ? true : config.mode === "light" ? false : systemDark;
  const effectiveAccentHex =
    config.source === "system"
      ? normalizeHex(systemAccentHex, DEFAULT_ACCENT_HEX)
      : normalizeHex(config.accentHex, DEFAULT_ACCENT_HEX);

  const scale = generateAccentScale(effectiveAccentHex);
  const generated = generatePalette(effectiveAccentHex, resolvedDark);
  const resolved: ThemePalette = { ...generated };

  for (const key of Object.keys(config.overrides) as ThemeRoleKey[]) {
    const val = config.overrides[key];
    if (val && /^#[0-9a-fA-F]{6}$/.test(val)) {
      resolved[key] = val.toUpperCase();
    }
  }

  const opacity = Math.min(1, Math.max(0.55, config.acrylicOpacity ?? 0.92));
  const darkIsland = config.islandSurface === "always_dark" || resolvedDark;
  const islandSourceHex =
    config.islandSurface === "always_dark" && !resolvedDark
      ? generatePalette(effectiveAccentHex, true)["--yuzey"]
      : resolved["--yuzey"];
  const [ir, ig, ib] = hexToRgb(islandSourceHex);
  const adaZemin = `rgba(${Math.round(ir * 255)}, ${Math.round(ig * 255)}, ${Math.round(ib * 255)}, ${opacity.toFixed(2)})`;
  const adaMetin = darkIsland ? "#F2F1F7" : resolved["--metin"];

  return {
    resolvedDark,
    effectiveAccentHex,
    scale,
    generated,
    resolved,
    adaZemin,
    adaMetin,
  };
}

export function applyThemeToDocument(
  theme: ResolvedTheme,
  fontSize: FontSizeOption = "md",
): void {
  const root = document.documentElement;
  root.dataset.theme = theme.resolvedDark ? "dark" : "light";
  root.style.colorScheme = theme.resolvedDark ? "dark" : "light";

  for (const [key, val] of Object.entries(theme.resolved)) {
    root.style.setProperty(key, val);
  }
  root.style.setProperty("--ada-zemin", theme.adaZemin);
  root.style.setProperty("--ada-metin", theme.adaMetin);
  const px = FONT_SIZE_PX[fontSize] ?? 14;
  root.style.setProperty("--font-size-base", `${px}px`);
  root.style.fontSize = `${px}px`;
}
