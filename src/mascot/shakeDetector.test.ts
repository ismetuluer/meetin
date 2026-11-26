import { describe, expect, it } from "vitest";
import { createShakeDetector } from "./shakeDetector";

describe("shakeDetector (2D)", () => {
  it("ignores steady dragging and tiny jitter", () => {
    const det = createShakeDetector();
    expect(det.push(100, 100, 0)).toBe(false);
    expect(det.push(130, 120, 50)).toBe(false);
    expect(det.push(180, 150, 100)).toBe(false);
    expect(det.push(240, 190, 150)).toBe(false);

    det.reset();
    expect(det.push(100, 100, 200)).toBe(false);
    expect(det.push(106, 105, 230)).toBe(false);
    expect(det.push(99, 98, 260)).toBe(false);
    expect(det.push(105, 104, 290)).toBe(false);
  });

  it("triggers on horizontal shake and respects cooldown", () => {
    const det = createShakeDetector();
    expect(det.push(100, 100, 0)).toBe(false);
    expect(det.push(130, 100, 60)).toBe(false);
    expect(det.push(100, 100, 120)).toBe(false);
    expect(det.push(132, 100, 180)).toBe(false);
    expect(det.push(98, 100, 240)).toBe(true);

    // Cooldown içinde tetiklemez
    expect(det.push(130, 100, 300)).toBe(false);
    expect(det.push(95, 100, 360)).toBe(false);
    expect(det.push(135, 100, 420)).toBe(false);
    expect(det.push(95, 100, 480)).toBe(false);
  });

  it("triggers on vertical and diagonal shaking too", () => {
    const vert = createShakeDetector();
    expect(vert.push(200, 100, 0)).toBe(false);
    expect(vert.push(200, 132, 60)).toBe(false); // aşağı
    expect(vert.push(200, 96, 120)).toBe(false); // yukarı (1)
    expect(vert.push(200, 134, 180)).toBe(false); // aşağı (2)
    expect(vert.push(200, 94, 240)).toBe(true); // yukarı (3) -> tetikler!

    const diag = createShakeDetector();
    expect(diag.push(100, 100, 0)).toBe(false);
    expect(diag.push(124, 124, 60)).toBe(false);
    expect(diag.push(96, 96, 120)).toBe(false);
    expect(diag.push(126, 126, 180)).toBe(false);
    expect(diag.push(94, 94, 240)).toBe(true);
  });
});
