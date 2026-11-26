import { describe, expect, it } from "vitest";
import {
  contrastRatio,
  DEFAULT_THEME_CONFIG,
  generateAccentScale,
  generatePalette,
  PRESET_ACCENTS,
  resolveThemeTokens,
  SCALE_STEPS,
} from "./palette";

function mulberry32(seed: number) {
  let a = seed;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function assertPaletteContrast(hex: string, dark: boolean) {
  const p = generatePalette(hex, dark);
  expect(contrastRatio(p["--metin"], p["--zemin"]), `${hex} dark=${dark} metin/zemin`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--metin"], p["--yuzey"]), `${hex} dark=${dark} metin/yuzey`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--metin"], p["--yuzey-2"]), `${hex} dark=${dark} metin/yuzey2`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--metin-ikincil"], p["--zemin"]), `${hex} dark=${dark} ikincil/zemin`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--metin-ikincil"], p["--yuzey"]), `${hex} dark=${dark} ikincil/yuzey`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--metin-ikincil"], p["--yuzey-2"]), `${hex} dark=${dark} ikincil/yuzey2`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--vurgu-ustu"], p["--vurgu"]), `${hex} dark=${dark} vurguUstu/vurgu`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--vurgu"], p["--vurgu-zayif"]), `${hex} dark=${dark} vurgu/vurguZayif`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--vurgu"], p["--yuzey"]), `${hex} dark=${dark} vurgu/yuzey`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--basari"], p["--yuzey"]), `${hex} dark=${dark} basari/yuzey`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--uyari"], p["--yuzey"]), `${hex} dark=${dark} uyari/yuzey`).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(p["--hata"], p["--yuzey"]), `${hex} dark=${dark} hata/yuzey`).toBeGreaterThanOrEqual(4.5);
}

describe("OKLCH palet üretici (M6.1)", () => {
  it("hazır renklerin tamamında 11 basamak ve >= 4,5:1 karşıtlık üretir", () => {
    for (const preset of PRESET_ACCENTS) {
      const scale = generateAccentScale(preset.hex);
      expect(Object.keys(scale)).toHaveLength(SCALE_STEPS.length);
      assertPaletteContrast(preset.hex, false);
      assertPaletteContrast(preset.hex, true);
    }
  });

  it("rastgele 500 renkte açık ve koyu temada >= 4,5:1 karşıtlığı korur", () => {
    const rand = mulberry32(20261126);
    for (let i = 0; i < 500; i++) {
      const r = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, "0");
      const g = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, "0");
      const b = Math.floor(rand() * 256)
        .toString(16)
        .padStart(2, "0");
      const hex = `#${r}${g}${b}`;
      assertPaletteContrast(hex, false);
      assertPaletteContrast(hex, true);
    }
  });

  it("elle değiştirilen rolleri ana renk değiştiğinde bile korur", () => {
    const base = resolveThemeTokens(
      {
        ...DEFAULT_THEME_CONFIG,
        source: "custom",
        accentHex: "#0078D4",
        overrides: { "--vurgu-hover": "#112233" },
      },
      false,
      "#0078D4",
    );
    expect(base.resolved["--vurgu-hover"]).toBe("#112233");

    const updated = resolveThemeTokens(
      {
        ...DEFAULT_THEME_CONFIG,
        source: "custom",
        accentHex: "#D6336C",
        overrides: { "--vurgu-hover": "#112233" },
      },
      false,
      "#0078D4",
    );
    expect(updated.resolved["--vurgu-hover"]).toBe("#112233");
    expect(updated.resolved["--vurgu"]).not.toBe(base.resolved["--vurgu"]);
  });
});
