import { describe, expect, it } from "vitest";

// Bileşen ve stil dosyaları (maskot çizimi, palet üretici ve tokens.css hariç)
const sources = import.meta.glob(
  [
    "/src/**/*.{ts,tsx,css}",
    "!/src/mascot/**",
    "!/src/**/*.test.*",
    "!/src/design/palette.ts",
    "!/src/design/tokens.css",
    "!/src/zz-*",
  ],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

const HARD_COLOR = /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/;

describe("renk rolleri (R3.5, A5)", () => {
  it("bileşenlerde sabit renk kalmaz; renkler §11.4 rolleri ya da tokens.css değişkenleridir", () => {
    const files = Object.keys(sources);
    expect(files.length).toBeGreaterThan(30);
    const offenders = files.flatMap((f) =>
      sources[f]
        .split("\n")
        .map((line, i) => ({ f, i: i + 1, line }))
        // "#" ile başlayan kimlik/bağlantı ve yorumlar değil, gerçek renk değerleri
        .filter(({ line }) =>
          HARD_COLOR.test(line.replace(/\/\/.*$|\/\*.*?\*\//g, "")),
        ),
    );
    expect(
      offenders.map(({ f, i, line }) => `${f}:${i}: ${line.trim()}`),
    ).toEqual([]);
  });
});
