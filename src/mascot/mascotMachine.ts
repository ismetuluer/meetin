/**
 * Meetin koyunu animasyon durum makinesi ve yardımcıları (§5.1.1).
 * Referans: docs/meetin-maskot-animasyon.html (Harman B · k-b2).
 */

import baaMp3Url from "../assets/sounds/baaa.mp3";
import whooshMp3Url from "../assets/sounds/dragon-studio-simple-whoosh-382724.mp3";
import punchClassicMp3Url from "../assets/sounds/universfield-classic-punch-impact-352711.mp3";
import thudImpactMp3Url from "../assets/sounds/universfield-impact-thud-291047.mp3";

export type MascotAnim =
  | "idle"
  | "graze"
  | "baa"
  | "stare"
  | "sleep"
  | "eat"
  | "hop"
  | "sad"
  | "squeeze"
  | "thumbsup"
  | "super";

export const ANIM_DURATIONS: Record<MascotAnim, number | null> = {
  idle: null,
  graze: 4200,
  baa: 1500,
  stare: 2800,
  sleep: null, // Durdurulana kadar sürer
  eat: 1700,
  hop: 1600,
  sad: 1500,
  squeeze: null, // Basılı tutulduğu sürece sürer
  thumbsup: 2200,
  super: null, // Uçuş bitene kadar sürer
};

export interface MascotFlags {
  eyeOpen: boolean;
  eyeClosed: boolean;
  eyeHappy: boolean;
  eyeSad: boolean;
  mSmile: boolean;
  mOpen: boolean;
  mChew: boolean;
  mSad: boolean;
  mGrin: boolean;
  isSleep: boolean;
  isBaa: boolean;
  isThumbsUp: boolean;
  isSuper: boolean;
  cheekOpacity: number;
}

export function getMascotFlags(anim: MascotAnim): MascotFlags {
  return {
    eyeOpen: !["sleep", "hop", "sad"].includes(anim),
    eyeClosed: anim === "sleep",
    eyeHappy: anim === "hop",
    eyeSad: anim === "sad",
    mSmile: ["idle", "stare", "sleep", "squeeze", "super"].includes(anim),
    mOpen: anim === "baa" || anim === "hop",
    mChew: anim === "graze" || anim === "eat",
    mSad: anim === "sad",
    mGrin: anim === "thumbsup",
    isSleep: anim === "sleep",
    isBaa: anim === "baa",
    isThumbsUp: anim === "thumbsup",
    isSuper: anim === "super",
    cheekOpacity: ["eat", "hop", "baa", "thumbsup", "super"].includes(anim)
      ? 0.95
      : 0.5,
  };
}

const AUTO_POOL: MascotAnim[] = [
  "graze",
  "graze",
  "graze",
  "stare",
  "stare",
  "hop",
  "baa",
];

/**
 * "Kendi hâlinde" modu için rastgele animasyon seçer (§5.1.1).
 */
export function pickAutoAnim(randomValue: number = Math.random()): MascotAnim {
  const clamped = Math.max(0, Math.min(0.999999, randomValue));
  return AUTO_POOL[Math.floor(clamped * AUTO_POOL.length)];
}

/**
 * İmleç farkından göz bebeği çevirme vektörünü hesaplar (tek `translate`, §5.1.1).
 */
export function computePupilOffset(
  dx: number,
  dy: number,
): { x: number; y: number } {
  const dist = Math.hypot(dx, dy) || 1;
  const mag = Math.min(dist / 60, 1) * 4.2;
  return {
    x: Number(((dx / dist) * mag).toFixed(1)),
    y: Number(((dy / dist) * mag).toFixed(1)),
  };
}

function playMp3(url: string, volume = 0.72): void {
  try {
    if (typeof Audio === "undefined") return;
    const audio = new Audio(url);
    audio.volume = Math.max(0.05, Math.min(1, volume));
    void audio.play().catch(() => {});
  } catch {
    // Ses aygıtı yoksa sessizce yok say
  }
}

/**
 * Kullanıcı tarafından sağlanan gerçek koyun sesi efekti (`baaa.mp3`, varsayılan kapalı).
 */
export function playBaaSound(): void {
  playMp3(baaMp3Url, 0.72);
}

/**
 * Fırlatma anında gerçek hava vınlaması (whoosh) ses efekti çalar (§5.1.1).
 */
export function playThrowSound(): void {
  playMp3(whooshMp3Url, 0.66);
}

const WALL_IMPACT_SOUNDS = [
  punchClassicMp3Url,
  thudImpactMp3Url,
];

/**
 * Duvara çarpma anında darbe/tok ses efekti çalar.
 */
export function playWallImpactSound(impactSpeed = 800): void {
  const poolIndex = Math.floor(Math.random() * WALL_IMPACT_SOUNDS.length);
  const url = WALL_IMPACT_SOUNDS[poolIndex] ?? thudImpactMp3Url;
  const vol = Math.min(0.84, Math.max(0.36, impactSpeed / 1600));
  playMp3(url, vol);
}
