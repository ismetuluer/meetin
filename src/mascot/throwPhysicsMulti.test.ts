import { describe, expect, it } from "vitest";
import {
  boundsOn,
  canCrossEdge,
  hostMonitorIndex,
  predictCrossesMonitor,
  stepThrowPhysics,
  stepThrowPhysicsMulti,
  type MonitorLayout,
  type MonitorRect,
  type ThrowState,
} from "./throwPhysics";

const A: MonitorRect = { x: 0, y: 0, width: 1920, height: 1080, scale: 1 };
/** Sağda, üstleri hizalı, daha yüksek ve %150 ölçekli monitör */
const B: MonitorRect = { x: 1920, y: 0, width: 2560, height: 1440, scale: 1.5 };

const layout = (...monitors: MonitorRect[]): MonitorLayout => ({
  monitors,
  windowWidth: 100,
  windowHeight: 100,
});

function run(
  state: ThrowState,
  l: MonitorLayout,
  gravity = 0,
  maxSteps = 2000,
) {
  let sim = state;
  let last = stepThrowPhysicsMulti(sim, l, 1 / 60, { gravity });
  const monitors = new Set<number>([hostMonitorIndex(state.x, state.y, l)]);
  for (let i = 0; i < maxSteps && !last.stopped; i++) {
    sim = last.state;
    monitors.add(last.monitor);
    last = stepThrowPhysicsMulti(sim, l, 1 / 60, { gravity });
  }
  monitors.add(last.monitor);
  return { end: last, monitors };
}

describe("çoklu monitör fırlatma (R3.0f)", () => {
  it("tek monitörde eski fizikle birebir aynıdır", () => {
    const l = layout(A);
    const s = { x: 1810, y: 970, vx: 2000, vy: 1500 };
    const multi = stepThrowPhysicsMulti(s, l, 0.02, { gravity: 1150 });
    const single = stepThrowPhysics(s, boundsOn(A, l), 0.02, { gravity: 1150 });
    expect(multi.state).toEqual(single.state);
    expect(multi.bounced).toBe(true);
    expect(multi.monitor).toBe(0);
  });

  it("pencere boyutu monitörün DPI ölçeğine göre hesaplanır", () => {
    const l = layout(A, B);
    expect(boundsOn(A, l).maxX).toBe(1820);
    expect(boundsOn(B, l)).toEqual({
      minX: 1920,
      minY: 0,
      maxX: 4330,
      maxY: 1290,
    });
  });

  it("bitişik monitöre duvara çarpmadan geçer ve orada durur", () => {
    const l = layout(A, B);
    const { end, monitors } = run({ x: 1500, y: 200, vx: 2400, vy: 0 }, l);
    expect(monitors.has(1)).toBe(true);
    expect(end.monitor).toBe(1);
    expect(end.state.x).toBeGreaterThan(1920);
    expect(end.state.x).toBeLessThanOrEqual(boundsOn(B, l).maxX);
  });

  it("komşu monitör pencereyi tamamen almıyorsa kenar duvardır", () => {
    const lowB = { ...B, y: 400 };
    const l = layout(A, lowB);
    expect(canCrossEdge(0, "right", 1820, 100, l)).toBe(false);
    expect(canCrossEdge(0, "right", 1820, 500, l)).toBe(true);
    const hit = stepThrowPhysicsMulti(
      { x: 1815, y: 100, vx: 2000, vy: 0 },
      l,
      0.02,
    );
    expect(hit.bounced).toBe(true);
    expect(hit.state.x).toBe(1820);
    expect(hit.state.vx).toBeLessThan(0);
  });

  it("aradaki boşluk duvardır; boşluktaki pencere en yakın monitöre yerleşir", () => {
    const gapped = { ...B, x: 2000 };
    const l = layout(A, gapped);
    expect(canCrossEdge(0, "right", 1820, 100, l)).toBe(false);
    const { end, monitors } = run({ x: 1500, y: 200, vx: 2400, vy: 0 }, l);
    expect(monitors.has(1)).toBe(false);
    expect(end.state.x).toBeLessThanOrEqual(1820);
    // Merkezi boşlukta (A için x≈1930): A kenarına daha yakın, içine çekilir
    const placed = stepThrowPhysicsMulti(
      { x: 1880, y: 200, vx: 0, vy: 0 },
      l,
      0.016,
    );
    expect(placed.monitor).toBe(0);
    expect(placed.state.x).toBe(1820);
  });

  it("basket modunda zemin maskotun altındaki monitörün tabanıdır", () => {
    const l = layout(A, B);
    const onB = run({ x: 3000, y: 100, vx: 0, vy: 0 }, l, 1150).end;
    expect(onB.monitor).toBe(1);
    expect(onB.state.y).toBeGreaterThan(1280);
    expect(onB.state.y).toBeLessThanOrEqual(1290);
    const onA = run({ x: 500, y: 100, vx: 0, vy: 0 }, l, 1150).end;
    expect(onA.state.y).toBeGreaterThan(970);
    expect(onA.state.y).toBeLessThanOrEqual(980);
  });

  it("alttaki monitöre düşerek geçer", () => {
    const below: MonitorRect = {
      x: 0,
      y: 1080,
      width: 1920,
      height: 1080,
      scale: 1,
    };
    const l = layout(A, below);
    const { end } = run({ x: 500, y: 100, vx: 0, vy: 0 }, l, 1150);
    expect(end.monitor).toBe(1);
    expect(end.state.y).toBeGreaterThan(2050);
  });

  it("roket yalnız atış başka monitöre ulaşacaksa seçilir", () => {
    const two = layout(A, B);
    expect(
      predictCrossesMonitor({ x: 1500, y: 200, vx: 2400, vy: 0 }, two),
    ).toBe(true);
    expect(predictCrossesMonitor({ x: 200, y: 200, vx: 400, vy: 0 }, two)).toBe(
      false,
    );
    expect(
      predictCrossesMonitor({ x: 1500, y: 200, vx: -2400, vy: 0 }, two),
    ).toBe(false);
    expect(
      predictCrossesMonitor({ x: 1500, y: 200, vx: 2400, vy: 0 }, layout(A)),
    ).toBe(false);
  });

  it("geçiş sürerken kenar kapanmaz, pencere hiçbir adımda sıçramaz", () => {
    const l = layout(A, B);
    const cases: Array<[ThrowState, number]> = [
      [{ x: 1930, y: 900, vx: -600, vy: 400 }, 0],
      [{ x: 1930, y: 700, vx: -900, vy: 0 }, 1150],
      [{ x: 1700, y: 850, vx: 900, vy: 300 }, 0],
      [{ x: 1700, y: 300, vx: 1600, vy: -200 }, 1150],
    ];
    for (const [start, gravity] of cases) {
      let sim = start;
      for (let i = 0; i < 600; i++) {
        const dt = 1 / 60;
        const step = stepThrowPhysicsMulti(sim, l, dt, { gravity });
        const moved = Math.hypot(step.state.x - sim.x, step.state.y - sim.y);
        const limit = Math.hypot(sim.vx, sim.vy + gravity * dt) * dt + 1;
        expect(moved).toBeLessThanOrEqual(limit);
        sim = step.state;
        if (step.stopped) break;
      }
    }
  });
});
