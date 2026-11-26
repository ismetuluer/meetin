export interface MonitorBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

export interface HoopRimRect {
  left: number;
  right: number;
  y: number;
}

export interface ThrowState {
  x: number;
  y: number;
  vx: number;
  vy: number;
}

export type ThrowSpeedTier = "none" | "gentle" | "fast" | "rocket";

export interface ThrowStepOptions {
  gravity?: number;
}

export interface ThrowStepResult {
  state: ThrowState;
  bounced: boolean;
  stopped: boolean;
  impactSpeed: number;
}

export interface VelocityTracker {
  sample: (x: number, y: number, nowMs: number) => void;
  getVelocity: (nowMs: number) => { vx: number; vy: number; speed: number };
  reset: () => void;
}

const MAX_THROW_SPEED = 3600;

/**
 * Sürükleme sırasındaki konum örneklerinden bırakma anındaki hızı (px/sn) hesaplar.
 */
export function createVelocityTracker(windowMs = 110): VelocityTracker {
  let samples: Array<{ x: number; y: number; t: number }> = [];

  function reset() {
    samples = [];
  }

  function sample(x: number, y: number, nowMs: number) {
    samples.push({ x, y, t: nowMs });
    const cutoff = nowMs - windowMs;
    while (samples.length > 2 && samples[0].t < cutoff) {
      samples.shift();
    }
  }

  function getVelocity(nowMs: number) {
    const cutoff = nowMs - windowMs;
    const recent = samples.filter((s) => s.t >= cutoff);
    if (recent.length < 2) {
      return { vx: 0, vy: 0, speed: 0 };
    }
    const first = recent[0];
    const last = recent[recent.length - 1];
    const dtSec = (last.t - first.t) / 1000;
    if (dtSec <= 0.004) {
      return { vx: 0, vy: 0, speed: 0 };
    }

    let vx = (last.x - first.x) / dtSec;
    let vy = (last.y - first.y) / dtSec;
    let speed = Math.hypot(vx, vy);
    if (speed > MAX_THROW_SPEED) {
      const scale = MAX_THROW_SPEED / speed;
      vx *= scale;
      vy *= scale;
      speed = MAX_THROW_SPEED;
    }
    return { vx, vy, speed };
  }

  return { sample, getVelocity, reset };
}

/**
 * Bırakma hızına göre maskot tepki kademesini belirler.
 */
export function classifyThrowSpeed(speed: number): ThrowSpeedTier {
  if (speed < 220) return "none";
  if (speed < 900) return "gentle";
  if (speed < 1800) return "fast";
  return "rocket";
}

/**
 * İvmeli sürükle-fırlat (isteğe bağlı yerçekimli) ve ekran kenar/köşelerinden sekme adımı.
 */
export function stepThrowPhysics(
  state: ThrowState,
  bounds: MonitorBounds,
  dtSec: number,
  options: ThrowStepOptions = {},
): ThrowStepResult {
  const dt = Math.max(0.001, Math.min(dtSec, 0.05));
  const gravity = options.gravity ?? 0;

  let vx = state.vx;
  let vy = state.vy + gravity * dt;
  let x = state.x + vx * dt;
  let y = state.y + vy * dt;

  const restitution = gravity > 0 ? 0.68 : 0.74;
  let bounced = false;
  let impactSpeed = 0;

  if (x <= bounds.minX) {
    x = bounds.minX;
    if (vx < 0) {
      impactSpeed = Math.max(impactSpeed, Math.abs(vx));
      vx = -vx * restitution;
      bounced = true;
    }
  } else if (x >= bounds.maxX) {
    x = bounds.maxX;
    if (vx > 0) {
      impactSpeed = Math.max(impactSpeed, Math.abs(vx));
      vx = -vx * restitution;
      bounced = true;
    }
  }

  if (y <= bounds.minY) {
    y = bounds.minY;
    if (vy < 0) {
      impactSpeed = Math.max(impactSpeed, Math.abs(vy));
      vy = -vy * restitution;
      bounced = true;
    }
  } else if (y >= bounds.maxY) {
    y = bounds.maxY;
    if (vy > 0) {
      impactSpeed = Math.max(impactSpeed, Math.abs(vy));
      vy = -vy * restitution;
      bounced = true;
    }
  }

  // Üstel sürtünme (basketbol modunda havada daha akıcı süzülme)
  const damping = Math.exp((gravity > 0 ? -0.45 : -2.5) * dt);
  vx *= damping;
  vy *= damping;

  const speed = Math.hypot(vx, vy);
  const onFloor = gravity > 0 ? y >= bounds.maxY - 2 : true;
  const stopped = onFloor && speed < (gravity > 0 ? 75 : 35);
  if (stopped) {
    vx = 0;
    vy = 0;
  }

  return {
    state: { x, y, vx, vy },
    bounced,
    stopped,
    impactSpeed,
  };
}

/**
 * Koyunun merkezi yukarıdan aşağıya doğru pota çemberinin içinden geçerse true döner.
 */
export function checkBasketScore(
  prevCenterX: number,
  prevCenterY: number,
  nextCenterX: number,
  nextCenterY: number,
  rim: HoopRimRect,
): boolean {
  if (prevCenterY > rim.y || nextCenterY < rim.y) {
    return false;
  }
  const dy = nextCenterY - prevCenterY;
  if (dy <= 0) {
    return false;
  }
  const frac = (rim.y - prevCenterY) / dy;
  const crossX = prevCenterX + (nextCenterX - prevCenterX) * frac;
  return crossX >= rim.left && crossX <= rim.right;
}
