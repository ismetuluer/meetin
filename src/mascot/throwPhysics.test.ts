import { describe, expect, it } from "vitest";
import {
  checkBasketScore,
  classifyThrowSpeed,
  createVelocityTracker,
  stepThrowPhysics,
  type MonitorBounds,
} from "./throwPhysics";

const BOUNDS: MonitorBounds = { minX: 0, minY: 0, maxX: 1820, maxY: 980 };

describe("throwPhysics", () => {
  it("tracks drag velocity and classifies speed tiers", () => {
    const tracker = createVelocityTracker(120);
    tracker.sample(100, 100, 0);
    tracker.sample(220, 100, 50); // 2400 px/sn

    const v = tracker.getVelocity(60);
    expect(Math.round(v.vx)).toBe(2400);
    expect(v.vy).toBe(0);
    expect(classifyThrowSpeed(v.speed)).toBe("rocket");

    expect(classifyThrowSpeed(120)).toBe("none");
    expect(classifyThrowSpeed(500)).toBe("gentle");
    expect(classifyThrowSpeed(1300)).toBe("fast");
  });

  it("bounces off screen edges and corners with restitution and stops via friction", () => {
    let state = { x: 1800, y: 20, vx: 2000, vy: -1800 };
    const hit = stepThrowPhysics(state, BOUNDS, 0.02);

    expect(hit.bounced).toBe(true);
    expect(hit.state.x).toBe(1820);
    expect(hit.state.y).toBe(0);
    expect(hit.state.vx).toBeLessThan(0);
    expect(hit.state.vy).toBeGreaterThan(0);
    expect(hit.impactSpeed).toBeGreaterThanOrEqual(1800);

    state = hit.state;
    let stopped = false;
    for (let i = 0; i < 200; i++) {
      const next = stepThrowPhysics(state, BOUNDS, 0.02);
      state = next.state;
      if (next.stopped) {
        stopped = true;
        break;
      }
    }
    expect(stopped).toBe(true);
  });

  it("applies gravity parabola and detects downward basket crossings only", () => {
    const rim = { left: 1000, right: 1100, y: 400 };
    // Yukarıdan aşağı çemberden geçiş -> basket!
    expect(checkBasketScore(1040, 385, 1055, 415, rim)).toBe(true);
    // Alttan yukarı geçiş -> basket değil
    expect(checkBasketScore(1040, 415, 1055, 385, rim)).toBe(false);
    // Çember dışından geçiş -> basket değil
    expect(checkBasketScore(940, 385, 955, 415, rim)).toBe(false);

    // Yerçekimi uygulanınca dikey hız aşağı yönde artar
    const res = stepThrowPhysics(
      { x: 500, y: 500, vx: 400, vy: -200 },
      BOUNDS,
      0.02,
      { gravity: 1150 },
    );
    expect(res.state.vy).toBeGreaterThan(-200);
  });
});
