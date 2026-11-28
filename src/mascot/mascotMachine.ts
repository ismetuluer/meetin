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
  | "super"
  | "rocket";

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
  rocket: null, // Monitörler arası uçuş bitene kadar sürer (R3.0f)
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
  /** Roketle monitörler arası uçuş (R3.0f) */
  isRocket: boolean;
  /** Uçuş duruşu (uzanmış toynaklar, eğik baş): `super` ve `rocket` */
  isFlight: boolean;
  cheekOpacity: number;
}

export function getMascotFlags(anim: MascotAnim): MascotFlags {
  return {
    eyeOpen: !["sleep", "hop", "sad"].includes(anim),
    eyeClosed: anim === "sleep",
    eyeHappy: anim === "hop",
    eyeSad: anim === "sad",
    mSmile: ["idle", "stare", "sleep", "squeeze", "super"].includes(anim),
    mOpen: anim === "baa" || anim === "hop" || anim === "rocket",
    mChew: anim === "graze" || anim === "eat",
    mSad: anim === "sad",
    mGrin: anim === "thumbsup",
    isSleep: anim === "sleep",
    isBaa: anim === "baa",
    isThumbsUp: anim === "thumbsup",
    isSuper: anim === "super",
    isRocket: anim === "rocket",
    isFlight: anim === "super" || anim === "rocket",
    cheekOpacity: ["eat", "hop", "baa", "thumbsup", "super", "rocket"].includes(anim)
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

let rocketAudio: AudioContext | null = null;

/**
 * Roket "fışş" sesi (R3.0f): süzgeçten geçmiş kısa beyaz gürültü, Web Audio ile üretilir.
 * Fırlatma sesi ayarına bağlıdır; çağıran ayarı denetler.
 */
export function playRocketSound(): void {
  try {
    if (typeof AudioContext === "undefined") return;
    rocketAudio ??= new AudioContext();
    const ctx = rocketAudio;
    void ctx.resume();
    const dur = 0.7;
    const buffer = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * dur), ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.Q.value = 0.9;
    const t0 = ctx.currentTime;
    filter.frequency.setValueAtTime(3200, t0);
    filter.frequency.exponentialRampToValueAtTime(700, t0 + dur);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, t0);
    gain.gain.exponentialRampToValueAtTime(0.5, t0 + 0.04);
    gain.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    src.connect(filter).connect(gain).connect(ctx.destination);
    src.start(t0);
    src.stop(t0 + dur);
  } catch {
    // Ses aygıtı yoksa sessizce yok say (diğer maskot sesleriyle aynı)
  }
}
