import { describe, expect, it } from "vitest";
import {
  ANIM_DURATIONS,
  computePupilOffset,
  getMascotFlags,
  pickAutoAnim,
  type MascotAnim,
} from "./mascotMachine";

describe("mascotMachine", () => {
  it("produces expected facial flags for all 11 catalog states", () => {
    const idle = getMascotFlags("idle");
    expect(idle.eyeOpen).toBe(true);
    expect(idle.mSmile).toBe(true);
    expect(idle.cheekOpacity).toBe(0.5);

    const sleep = getMascotFlags("sleep");
    expect(sleep.eyeClosed).toBe(true);
    expect(sleep.eyeOpen).toBe(false);
    expect(sleep.isSleep).toBe(true);

    const hop = getMascotFlags("hop");
    expect(hop.eyeHappy).toBe(true);
    expect(hop.mOpen).toBe(true);
    expect(hop.cheekOpacity).toBe(0.95);

    const eat = getMascotFlags("eat");
    expect(eat.mChew).toBe(true);
    expect(eat.cheekOpacity).toBe(0.95);

    const sad = getMascotFlags("sad");
    expect(sad.eyeSad).toBe(true);
    expect(sad.mSad).toBe(true);

    const thumbs = getMascotFlags("thumbsup");
    expect(thumbs.isThumbsUp).toBe(true);
    expect(thumbs.mGrin).toBe(true);
    expect(thumbs.cheekOpacity).toBe(0.95);

    const sup = getMascotFlags("super");
    expect(sup.isSuper).toBe(true);
    expect(sup.eyeOpen).toBe(true);
    expect(sup.cheekOpacity).toBe(0.95);
    expect(sup.isFlight).toBe(true);
    expect(sup.isRocket).toBe(false);

    const rocket = getMascotFlags("rocket");
    expect(rocket.isRocket).toBe(true);
    expect(rocket.isFlight).toBe(true);
    expect(rocket.isSuper).toBe(false);
    expect(rocket.mOpen).toBe(true);
  });

  it("defines durations matching spec section 5.1.1", () => {
    expect(ANIM_DURATIONS.graze).toBe(4200);
    expect(ANIM_DURATIONS.baa).toBe(1500);
    expect(ANIM_DURATIONS.stare).toBe(2800);
    expect(ANIM_DURATIONS.eat).toBe(1700);
    expect(ANIM_DURATIONS.hop).toBe(1600);
    expect(ANIM_DURATIONS.sad).toBe(1500);
    expect(ANIM_DURATIONS.thumbsup).toBe(2200);
    expect(ANIM_DURATIONS.sleep).toBeNull();
    expect(ANIM_DURATIONS.squeeze).toBeNull();
    expect(ANIM_DURATIONS.super).toBeNull();
    expect(ANIM_DURATIONS.rocket).toBeNull();
  });

  it("clamps pupil tracking vector to 4.2px max radius", () => {
    const far = computePupilOffset(600, 0);
    expect(far.x).toBe(4.2);
    expect(far.y).toBe(0);

    const zero = computePupilOffset(0, 0);
    expect(zero.x).toBe(0);
    expect(zero.y).toBe(0);
  });

  it("picks only valid autopilot animations", () => {
    const allowed: MascotAnim[] = ["graze", "baa", "stare", "hop"];
    for (const v of [0, 0.25, 0.5, 0.75, 0.99]) {
      expect(allowed).toContain(pickAutoAnim(v));
    }
  });
});
