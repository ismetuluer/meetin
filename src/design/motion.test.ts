import { describe, expect, it } from "vitest";
import tokensCss from "./tokens.css?raw";
import {
  AZ_HAREKET_SURE_MS,
  motionCssTokens,
  settleTime,
  springLinearEasing,
  springValue,
  YAY,
  type SpringConfig,
} from "./motion";

describe("yay çözücü (R3.0, şartname §11.5)", () => {
  it("şartnamenin temel değerlerini kullanır: sertlik 380, sönüm 28", () => {
    expect(YAY.stiffness).toBe(380);
    expect(YAY.damping).toBe(28);
  });

  it("0'dan başlar, 1'e oturur", () => {
    expect(springValue(YAY, 0)).toBeCloseTo(0, 10);
    expect(springValue(YAY, 2)).toBeCloseTo(1, 6);
  });

  it("380/28 hafifçe taşar (%3–5) ve tepe ~230 ms'de olur", () => {
    let peak = 0;
    let peakAt = 0;
    for (let t = 0; t <= 1; t += 0.001) {
      const v = springValue(YAY, t);
      if (v > peak) {
        peak = v;
        peakAt = t;
      }
    }
    expect(peak - 1).toBeGreaterThan(0.03);
    expect(peak - 1).toBeLessThan(0.05);
    expect(peakAt).toBeGreaterThan(0.2);
    expect(peakAt).toBeLessThan(0.26);
  });

  it("kritik ve aşırı sönümlü yaylar hiç taşmaz ve 1'e oturur", () => {
    const critical: SpringConfig = { stiffness: 400, damping: 40 };
    const over: SpringConfig = { stiffness: 400, damping: 80 };
    for (const cfg of [critical, over]) {
      let prev = 0;
      for (let t = 0; t <= 3; t += 0.005) {
        const v = springValue(cfg, t);
        expect(v).toBeLessThanOrEqual(1 + 1e-9);
        expect(v + 1e-9).toBeGreaterThanOrEqual(prev);
        prev = v;
      }
      expect(springValue(cfg, 5)).toBeCloseTo(1, 4);
    }
  });

  it("oturma süresi şartnamedeki ada açılışına (~350 ms) denk gelir", () => {
    const ms = settleTime(YAY) * 1000;
    expect(ms).toBeGreaterThan(320);
    expect(ms).toBeLessThan(360);
    // Oturduktan sonra %1 bandından bir daha çıkmaz.
    for (let t = settleTime(YAY); t < 2; t += 0.002) {
      expect(Math.abs(springValue(YAY, t) - 1)).toBeLessThanOrEqual(0.01 + 1e-9);
    }
  });

  it("CSS linear() eğrisi 0 ile başlar, 1 ile biter ve taşmayı korur", () => {
    const easing = springLinearEasing(YAY);
    expect(easing.startsWith("linear(")).toBe(true);
    const points = easing
      .slice("linear(".length, -1)
      .split(",")
      .map((p) => Number(p.trim()));
    expect(points.length).toBeGreaterThanOrEqual(24);
    expect(points.every((p) => Number.isFinite(p))).toBe(true);
    expect(points[0]).toBe(0);
    expect(points[points.length - 1]).toBe(1);
    expect(Math.max(...points)).toBeGreaterThan(1.03);
  });
});

describe("hareket tokenları", () => {
  const css = tokensCss.replace(/\r\n/g, "\n");

  it("tokens.css motion.ts'nin ürettiği değerleri taşır (elle değiştirilmez)", () => {
    for (const [name, value] of Object.entries(motionCssTokens())) {
      expect(css, `${name}: ${value};`).toContain(`${name}: ${value};`);
    }
  });

  it("maskot dışında kendi cubic-bezier eğrisini kullanan CSS kalmaz", () => {
    // Maskot animasyonları docs/meetin-maskot-animasyon.html'den birebir taşınır; dokunulmaz.
    const files = import.meta.glob<string>(
      ["../**/*.css", "../**/*.ts", "../**/*.tsx", "!../mascot/**", "!../**/*.test.ts"],
      { query: "?raw", import: "default", eager: true },
    );
    expect(Object.keys(files).length).toBeGreaterThan(10);
    const offenders = Object.entries(files)
      .filter(([, text]) => text.includes("cubic-bezier("))
      .map(([path]) => path);
    expect(offenders, "var(--yay-egri) kullanın").toEqual([]);
  });

  it("Windows'ta animasyonlar kapalıyken yay kısa düz geçişe döner", () => {
    const start = css.indexOf("@media (prefers-reduced-motion: reduce)");
    expect(start).toBeGreaterThanOrEqual(0);
    const reduced = css.slice(start);
    const block = reduced.slice(0, reduced.indexOf("}\n}") + 3);
    expect(block).toContain("--yay-egri: ease-out;");
    expect(block).toContain(`--yay-sure: ${AZ_HAREKET_SURE_MS}ms;`);
    expect(block).toContain(`--yay-sure-kisa: ${AZ_HAREKET_SURE_MS}ms;`);
  });
});
