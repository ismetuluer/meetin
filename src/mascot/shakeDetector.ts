export interface ShakeDetectorOptions {
  minDeltaPx?: number;
  windowMs?: number;
  requiredReversals?: number;
  cooldownMs?: number;
}

export interface ShakeDetector {
  push: (x: number, y: number, nowMs: number) => boolean;
  reset: () => void;
}

/**
 * Maskotu sürüklerken yatay, dikey veya çapraz fark etmeksizin her yöndeki
 * hızlı gidiş-geliş sallama hareketini algılar.
 */
export function createShakeDetector(
  opts: ShakeDetectorOptions = {},
): ShakeDetector {
  const minDeltaPx = opts.minDeltaPx ?? 14;
  const windowMs = opts.windowMs ?? 650;
  const requiredReversals = opts.requiredReversals ?? 3;
  const cooldownMs = opts.cooldownMs ?? 900;

  let anchorX: number | null = null;
  let anchorY: number | null = null;
  let lastUx = 0;
  let lastUy = 0;
  let hasDir = false;
  let reversals: number[] = [];
  let lastTriggerMs = -Infinity;

  function reset() {
    anchorX = null;
    anchorY = null;
    lastUx = 0;
    lastUy = 0;
    hasDir = false;
    reversals = [];
  }

  function push(x: number, y: number, nowMs: number): boolean {
    if (nowMs - lastTriggerMs < cooldownMs) {
      anchorX = x;
      anchorY = y;
      return false;
    }

    if (anchorX === null || anchorY === null) {
      anchorX = x;
      anchorY = y;
      return false;
    }

    const dx = x - anchorX;
    const dy = y - anchorY;
    const dist = Math.hypot(dx, dy);
    if (dist < minDeltaPx) {
      return false;
    }

    const ux = dx / dist;
    const uy = dy / dist;
    anchorX = x;
    anchorY = y;

    if (hasDir) {
      const dot = ux * lastUx + uy * lastUy;
      // ~110° ve üzeri yön dönüşleri sallama yön değişimi sayılır (her eksen/çapraz dahil)
      if (dot < -0.35) {
        reversals = reversals.filter((t) => nowMs - t <= windowMs);
        reversals.push(nowMs);
        lastUx = ux;
        lastUy = uy;

        if (reversals.length >= requiredReversals) {
          lastTriggerMs = nowMs;
          reset();
          return true;
        }
        return false;
      }
    }

    lastUx = ux;
    lastUy = uy;
    hasDir = true;
    return false;
  }

  return { push, reset };
}
