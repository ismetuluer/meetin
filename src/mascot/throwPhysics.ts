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

/** Bir monitörün sanal masaüstündeki fiziksel dikdörtgeni (Rust `MonitorRectDto`). */
export interface MonitorRect {
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
}

/** Bütün monitörler + maskot penceresinin mantıksal boyutu (R3.0f). */
export interface MonitorLayout {
  monitors: MonitorRect[];
  windowWidth: number;
  windowHeight: number;
}

export interface MultiThrowStepResult extends ThrowStepResult {
  /** Pencere merkezinin bulunduğu monitörün `layout.monitors` içindeki sırası. */
  monitor: number;
}

/** Monitörler arasındaki yuvarlama farkı için kenar toleransı (fiziksel px). */
const EDGE_TOLERANCE = 2;

/** Pencerenin o monitördeki fiziksel boyutu: Windows DPI değişince pencereyi yeniden boyutlar. */
export function windowSizeOn(
  m: MonitorRect,
  layout: MonitorLayout,
): { w: number; h: number } {
  return {
    w: Math.round(layout.windowWidth * m.scale),
    h: Math.round(layout.windowHeight * m.scale),
  };
}

/** Pencerenin sol üst köşesinin o monitörde gidebileceği sınırlar. */
export function boundsOn(m: MonitorRect, layout: MonitorLayout): MonitorBounds {
  const { w, h } = windowSizeOn(m, layout);
  return {
    minX: m.x,
    minY: m.y,
    maxX: Math.max(m.x, m.x + m.width - w),
    maxY: Math.max(m.y, m.y + m.height - h),
  };
}

/**
 * Pencere merkezinin bulunduğu monitör. Hiçbirinde değilse (monitörler arası boşluk,
 * ekran dışı) merkeze en yakın monitör döner.
 */
export function hostMonitorIndex(
  x: number,
  y: number,
  layout: MonitorLayout,
): number {
  let best = 0;
  let bestDist = Infinity;
  layout.monitors.forEach((m, i) => {
    const { w, h } = windowSizeOn(m, layout);
    const cx = x + w / 2;
    const cy = y + h / 2;
    const dx = Math.max(m.x - cx, 0, cx - (m.x + m.width));
    const dy = Math.max(m.y - cy, 0, cy - (m.y + m.height));
    const dist = Math.hypot(dx, dy);
    if (dist < bestDist) {
      bestDist = dist;
      best = i;
    }
  });
  return best;
}

type Edge = "left" | "right" | "top" | "bottom";

/**
 * Kenarın ötesinde pencereyi tamamen alacak bitişik bir monitör varsa o kenar duvar değildir.
 * Pencerenin kenar boyunca uzanan kısmı komşuya sığmıyorsa (farklı yükseklik/kaydırma) duvardır.
 */
export function canCrossEdge(
  hostIndex: number,
  edge: Edge,
  x: number,
  y: number,
  layout: MonitorLayout,
): boolean {
  const host = layout.monitors[hostIndex];
  const hostSize = windowSizeOn(host, layout);
  return layout.monitors.some((n, i) => {
    if (i === hostIndex) return false;
    const nSize = windowSizeOn(n, layout);
    const w = Math.max(hostSize.w, nSize.w);
    const h = Math.max(hostSize.h, nSize.h);
    const near = (a: number, b: number) => Math.abs(a - b) <= EDGE_TOLERANCE;
    const fitsY =
      n.y - EDGE_TOLERANCE <= y && y + h <= n.y + n.height + EDGE_TOLERANCE;
    const fitsX =
      n.x - EDGE_TOLERANCE <= x && x + w <= n.x + n.width + EDGE_TOLERANCE;
    switch (edge) {
      case "left":
        return near(n.x + n.width, host.x) && fitsY;
      case "right":
        return near(n.x, host.x + host.width) && fitsY;
      case "top":
        return near(n.y + n.height, host.y) && fitsX;
      case "bottom":
        return near(n.y, host.y + host.height) && fitsX;
    }
  });
}

/**
 * Pencere kenarı aşıp komşu monitöre taşmışsa (geçiş sürüyor) o komşu ve pencerenin
 * kenar boyunca kalabileceği ortak aralık. Geçiş sürerken kenar kapanmaz; pencere iki
 * monitörün ortak kısmında tutulur, böylece geri sıçramaz.
 */
function straddleSpan(
  hostIndex: number,
  edge: Edge,
  x: number,
  y: number,
  layout: MonitorLayout,
): [number, number] | null {
  const host = layout.monitors[hostIndex];
  const hostSize = windowSizeOn(host, layout);
  const near = (a: number, c: number) => Math.abs(a - c) <= EDGE_TOLERANCE;
  const horizontal = edge === "left" || edge === "right";
  let best: [number, number] | null = null;
  let bestOverlap = 0;
  layout.monitors.forEach((n, i) => {
    if (i === hostIndex) return;
    const nSize = windowSizeOn(n, layout);
    const w = Math.max(hostSize.w, nSize.w);
    const h = Math.max(hostSize.h, nSize.h);
    let adjacent: boolean;
    let straddling: boolean;
    switch (edge) {
      case "left":
        adjacent = near(n.x + n.width, host.x);
        straddling = x < host.x;
        break;
      case "right":
        adjacent = near(n.x, host.x + host.width);
        straddling = x + w > host.x + host.width;
        break;
      case "top":
        adjacent = near(n.y + n.height, host.y);
        straddling = y < host.y;
        break;
      case "bottom":
        adjacent = near(n.y, host.y + host.height);
        straddling = y + h > host.y + host.height;
        break;
    }
    if (!adjacent || !straddling) return;
    const [lo, hi, pos, size] = horizontal
      ? [
          Math.max(host.y, n.y),
          Math.min(host.y + host.height, n.y + n.height),
          y,
          h,
        ]
      : [
          Math.max(host.x, n.x),
          Math.min(host.x + host.width, n.x + n.width),
          x,
          w,
        ];
    const overlap = Math.min(pos + size, hi) - Math.max(pos, lo);
    if (hi - lo >= size && overlap > bestOverlap) {
      bestOverlap = overlap;
      best = [lo, hi - size];
    }
  });
  return best;
}

/**
 * Çoklu monitörde fırlatma adımı (R3.0f): duvarlar yalnız birleşik alanın dış kenarlarıdır,
 * zemin maskotun altındaki monitörün tabanıdır. Tek monitörde `stepThrowPhysics` ile aynıdır.
 */
export function stepThrowPhysicsMulti(
  state: ThrowState,
  layout: MonitorLayout,
  dtSec: number,
  options: ThrowStepOptions = {},
): MultiThrowStepResult {
  const host = hostMonitorIndex(state.x, state.y, layout);
  const b = boundsOn(layout.monitors[host], layout);
  const bounds: MonitorBounds = { ...b };
  const spans: Array<[Edge, [number, number]]> = [];
  for (const edge of ["left", "right", "top", "bottom"] as Edge[]) {
    const span = straddleSpan(host, edge, state.x, state.y, layout);
    if (span) spans.push([edge, span]);
    if (!span && !canCrossEdge(host, edge, state.x, state.y, layout)) continue;
    if (edge === "left") bounds.minX = -Infinity;
    if (edge === "right") bounds.maxX = Infinity;
    if (edge === "top") bounds.minY = -Infinity;
    if (edge === "bottom") bounds.maxY = Infinity;
  }
  // Geçiş sürerken pencere iki monitörün ortak aralığında kalır
  for (const [edge, [lo, hi]] of spans) {
    if (edge === "left" || edge === "right") {
      bounds.minY = Math.max(bounds.minY, lo);
      bounds.maxY = Math.min(bounds.maxY, hi);
    } else {
      bounds.minX = Math.max(bounds.minX, lo);
      bounds.maxX = Math.min(bounds.maxX, hi);
    }
  }
  const step = stepThrowPhysics(state, bounds, dtSec, options);
  return {
    ...step,
    monitor: hostMonitorIndex(step.state.x, step.state.y, layout),
  };
}

/**
 * Atış başka bir monitöre ulaşacak mı? Aynı fizik sabit adımla önceden çalıştırılır;
 * sonuç roket animasyonunu seçer (R3.0f).
 */
export function predictCrossesMonitor(
  initial: ThrowState,
  layout: MonitorLayout,
  options: ThrowStepOptions = {},
  maxSec = 6,
): boolean {
  if (layout.monitors.length < 2) return false;
  const start = hostMonitorIndex(initial.x, initial.y, layout);
  const dt = 1 / 60;
  let sim = initial;
  for (let t = 0; t < maxSec; t += dt) {
    const step = stepThrowPhysicsMulti(sim, layout, dt, options);
    if (step.monitor !== start) return true;
    if (step.stopped) return false;
    sim = step.state;
  }
  return false;
}
