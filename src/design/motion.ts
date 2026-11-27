/* Meetin ortak yay (spring) altyapısı (R3.0, şartname §11.5)
 *
 * Yay değerleri yalnız burada tanımlanır. CSS tarafı `tokens.css` içindeki
 * `--yay-egri`, `--yay-sure`, `--yay-sure-kisa` değişkenlerini kullanır; bu değerler
 * `motionCssTokens()` çıktısıdır ve `motion.test.ts` ikisinin aynı kaldığını denetler.
 * Yay değiştirilirse testin gösterdiği yeni değerler `tokens.css`'e yazılır.
 *
 * Kısa süre (`--yay-sure-kisa`) aynı eğriyi daha hızlı oynatır: zaman ölçeklenen yay
 * aynı sönüm oranını korur, yani aynı "yaylılık" daha sert bir yayla elde edilir.
 */

export interface SpringConfig {
  /** Sertlik (k) */
  stiffness: number;
  /** Sönüm (c) */
  damping: number;
  /** Kütle (m), varsayılan 1 */
  mass?: number;
}

/** Şartname §11.5 temel yayı. */
export const YAY: SpringConfig = { stiffness: 380, damping: 28, mass: 1 };

/** Kapanış ve küçük geçişler (şartname §11.5: kapanış ~220 ms). */
export const YAY_SURE_KISA_MS = 220;

/** `prefers-reduced-motion` açıkken yayların yerini alan düz geçiş süresi. */
export const AZ_HAREKET_SURE_MS = 120;

/**
 * Durgun hâlden (konum 0, hız 0) hedefe (1) giden sönümlü yayın t saniyedeki konumu.
 * Alt, kritik ve aşırı sönümlü durumların kapalı çözümü.
 */
export function springValue(cfg: SpringConfig, t: number): number {
  if (t <= 0) return 0;
  const m = cfg.mass ?? 1;
  const w0 = Math.sqrt(cfg.stiffness / m);
  const zeta = cfg.damping / (2 * Math.sqrt(cfg.stiffness * m));

  if (zeta < 1) {
    const wd = w0 * Math.sqrt(1 - zeta * zeta);
    const decay = Math.exp(-zeta * w0 * t);
    return 1 - decay * (Math.cos(wd * t) + ((zeta * w0) / wd) * Math.sin(wd * t));
  }
  if (zeta === 1) {
    return 1 - Math.exp(-w0 * t) * (1 + w0 * t);
  }
  const s = w0 * Math.sqrt(zeta * zeta - 1);
  const r1 = -zeta * w0 + s;
  const r2 = -zeta * w0 - s;
  // x(0) = 0, x'(0) = 0 koşullarıyla iki üstel terim
  return 1 - (r2 * Math.exp(r1 * t) - r1 * Math.exp(r2 * t)) / (r2 - r1);
}

/**
 * Yayın hedefin `epsilon` bandına girip bir daha çıkmadığı an (saniye).
 * 1 ms adımla 5 saniyeye kadar taranır.
 */
export function settleTime(cfg: SpringConfig, epsilon = 0.01): number {
  const step = 0.001;
  let last = 0;
  for (let t = 0; t <= 5; t += step) {
    if (Math.abs(springValue(cfg, t) - 1) > epsilon) last = t;
  }
  return last + step;
}

/** Yayın açılış süresi (ms): %1 bandına oturma anı, 10 ms'ye yuvarlanır. */
export function springDurationMs(cfg: SpringConfig): number {
  return Math.round((settleTime(cfg) * 1000) / 10) * 10;
}

/**
 * Yayı CSS `linear()` zamanlama fonksiyonuna çevirir (WebView2/Chromium 113+).
 * Eğri oturma süresi boyunca eşit aralıklarla örneklenir; son nokta tam 1'dir.
 */
export function springLinearEasing(cfg: SpringConfig, samples = 32): string {
  const duration = settleTime(cfg);
  const points: string[] = [];
  for (let i = 0; i < samples; i++) {
    const v = springValue(cfg, (duration * i) / (samples - 1));
    points.push(String(Math.round(v * 10000) / 10000));
  }
  points[0] = "0";
  points[samples - 1] = "1";
  return `linear(${points.join(", ")})`;
}

/** `tokens.css`'teki hareket değişkenlerinin beklenen değerleri. */
export function motionCssTokens(): Record<string, string> {
  return {
    "--yay-egri": springLinearEasing(YAY),
    "--yay-sure": `${springDurationMs(YAY)}ms`,
    "--yay-sure-kisa": `${YAY_SURE_KISA_MS}ms`,
  };
}
