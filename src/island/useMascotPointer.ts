import {
  useRef,
  type Dispatch,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
  type SetStateAction,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { MascotAnim } from "../mascot/mascotMachine";
import { createShakeDetector } from "../mascot/shakeDetector";
import {
  classifyThrowSpeed,
  createVelocityTracker,
  type ThrowState,
} from "../mascot/throwPhysics";
import type { IslandMode } from "./islandTypes";

interface MascotPointerDeps {
  paused: boolean;
  cancelThrow: () => void;
  startThrowLoop: (initial: ThrowState) => void;
  clearAnimTimer: () => void;
  setAnim: Dispatch<SetStateAction<MascotAnim>>;
  setFlightAngleDeg: Dispatch<SetStateAction<number>>;
  playAnim: (next: MascotAnim, allowBaaSound?: boolean) => void;
  setMode: Dispatch<SetStateAction<IslandMode>>;
  openSearch: () => Promise<void>;
  basketballModeRef: RefObject<boolean>;
  lastUserRef: RefObject<number>;
  isDraggingRef: RefObject<boolean>;
}

// Maskot fare etkileşimleri: tek tık / çift tık / basılı tut / sürükle-fırlat / salla / sağ tık (§5.1)
export function useMascotPointer({
  paused,
  cancelThrow,
  startThrowLoop,
  clearAnimTimer,
  setAnim,
  setFlightAngleDeg,
  playAnim,
  setMode,
  openSearch,
  basketballModeRef,
  lastUserRef,
  isDraggingRef,
}: MascotPointerDeps) {
  const shakeDetectorRef = useRef(createShakeDetector());
  const velocityTrackerRef = useRef(createVelocityTracker());

  // İşaretçi hareket / basılı tutma / tıklama ayrımı (maskot)
  const pointerStateRef = useRef<{
    down: boolean;
    pointerId: number | null;
    startClientX: number;
    startClientY: number;
    startScreenX: number;
    startScreenY: number;
    startWinX: number;
    startWinY: number;
    hasWinPos: boolean;
    lastSentX: number;
    lastSentY: number;
    dragStarted: boolean;
    heldLong: boolean;
    holdTimer: number | null;
    clickTimer: number | null;
  }>({
    down: false,
    pointerId: null,
    startClientX: 0,
    startClientY: 0,
    startScreenX: 0,
    startScreenY: 0,
    startWinX: 0,
    startWinY: 0,
    hasWinPos: false,
    lastSentX: 0,
    lastSentY: 0,
    dragStarted: false,
    heldLong: false,
    holdTimer: null,
    clickTimer: null,
  });

  function handleMascotPointerDown(e: ReactPointerEvent<HTMLButtonElement>) {
    if (e.button !== 0) return;
    cancelThrow();
    lastUserRef.current = Date.now();
    const ps = pointerStateRef.current;
    ps.down = true;
    ps.pointerId = e.pointerId;
    ps.startClientX = e.clientX;
    ps.startClientY = e.clientY;
    ps.startScreenX = e.screenX;
    ps.startScreenY = e.screenY;
    ps.hasWinPos = false;
    ps.dragStarted = false;
    ps.heldLong = false;
    shakeDetectorRef.current.reset();
    velocityTrackerRef.current.reset();

    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Yok say
    }

    void getCurrentWebviewWindow()
      .outerPosition()
      .then((pos) => {
        ps.startWinX = pos.x;
        ps.startWinY = pos.y;
        ps.lastSentX = pos.x;
        ps.lastSentY = pos.y;
        ps.hasWinPos = true;
        velocityTrackerRef.current.sample(pos.x, pos.y, performance.now());
      })
      .catch(() => {});

    clearAnimTimer();
    setAnim("squeeze");

    if (ps.holdTimer !== null) window.clearTimeout(ps.holdTimer);
    ps.holdTimer = window.setTimeout(() => {
      ps.heldLong = true;
    }, 450);
  }

  function handleMascotPointerMove(e: ReactPointerEvent<HTMLButtonElement>) {
    const ps = pointerStateRef.current;
    if (!ps.down) return;

    if (!ps.dragStarted) {
      const dist = Math.hypot(
        e.clientX - ps.startClientX,
        e.clientY - ps.startClientY,
      );
      if (dist <= 5) return;

      ps.dragStarted = true;
      isDraggingRef.current = true;
      if (ps.holdTimer !== null) {
        window.clearTimeout(ps.holdTimer);
        ps.holdTimer = null;
      }
      clearAnimTimer();
      setAnim("super");
    }

    if (!ps.hasWinPos) return;
    const now = performance.now();
    const dpr = window.devicePixelRatio || 1;
    const nextX = Math.round(ps.startWinX + (e.screenX - ps.startScreenX) * dpr);
    const nextY = Math.round(ps.startWinY + (e.screenY - ps.startScreenY) * dpr);
    const moveDx = nextX - ps.lastSentX;
    const moveDy = nextY - ps.lastSentY;
    if (moveDx !== 0 || moveDy !== 0) {
      if (Math.hypot(moveDx, moveDy) >= 2) {
        setFlightAngleDeg((Math.atan2(moveDy, moveDx) * 180) / Math.PI);
      }
      ps.lastSentX = nextX;
      ps.lastSentY = nextY;
      velocityTrackerRef.current.sample(nextX, nextY, now);
      void invoke("move_mascot_window", { x: nextX, y: nextY });
    }

    if (shakeDetectorRef.current.push(e.screenX, e.screenY, now)) {
      velocityTrackerRef.current.reset();
      playAnim("hop");
      void invoke<boolean>("toggle_studio_window");
    }
  }

  function finishMascotInteraction(
    target: HTMLButtonElement | null,
    treatAsClick: boolean,
  ) {
    const ps = pointerStateRef.current;
    if (!ps.down) return;
    ps.down = false;

    if (target && ps.pointerId !== null) {
      try {
        if (target.hasPointerCapture(ps.pointerId)) {
          target.releasePointerCapture(ps.pointerId);
        }
      } catch {
        // Yok say
      }
    }
    ps.pointerId = null;

    if (ps.holdTimer !== null) {
      window.clearTimeout(ps.holdTimer);
      ps.holdTimer = null;
    }

    if (ps.dragStarted) {
      ps.dragStarted = false;
      isDraggingRef.current = false;
      shakeDetectorRef.current.reset();

      const vel = velocityTrackerRef.current.getVelocity(performance.now());
      velocityTrackerRef.current.reset();
      const tier = treatAsClick ? classifyThrowSpeed(vel.speed) : "none";

      if (ps.hasWinPos && (tier !== "none" || basketballModeRef.current)) {
        const launchVx =
          tier === "none" ? vel.vx * 1.15 : vel.vx;
        const launchVy =
          tier === "none" ? vel.vy * 1.15 : vel.vy;
        if (Math.hypot(launchVx, launchVy) >= 140) {
          startThrowLoop({
            x: ps.lastSentX,
            y: ps.lastSentY,
            vx: launchVx,
            vy: launchVy,
          });
          return;
        }
      }

      if (ps.hasWinPos) {
        void invoke("save_mascot_position", {
          x: ps.lastSentX,
          y: ps.lastSentY,
        });
      }
      setAnim(paused ? "sleep" : "idle");
      return;
    }

    if (!treatAsClick) {
      setAnim(paused ? "sleep" : "idle");
      return;
    }

    if (ps.heldLong) {
      ps.heldLong = false;
      setAnim(paused ? "sleep" : "idle");
      void invoke("open_add_from_squeeze");
      return;
    }

    setAnim(paused ? "sleep" : "idle");

    // Tek tık ile çift tık ayrımı
    if (ps.clickTimer !== null) {
      window.clearTimeout(ps.clickTimer);
      ps.clickTimer = null;
      void invoke("open_studio_window");
    } else {
      ps.clickTimer = window.setTimeout(() => {
        ps.clickTimer = null;
        void openSearch();
      }, 220);
    }
  }

  function handleMascotPointerUp(e: ReactPointerEvent<HTMLButtonElement>) {
    if (e.button !== 0) return;
    finishMascotInteraction(e.currentTarget, true);
  }

  function handleMascotPointerCancel(e: ReactPointerEvent<HTMLButtonElement>) {
    finishMascotInteraction(e.currentTarget, false);
  }

  function handleMascotContextMenu(e: ReactMouseEvent) {
    e.preventDefault();
    lastUserRef.current = Date.now();
    setMode("menu");
    void invoke("set_mascot_mode", { mode: "menu" });
  }

  return {
    handleMascotPointerDown,
    handleMascotPointerMove,
    handleMascotPointerUp,
    handleMascotPointerCancel,
    handleMascotContextMenu,
  };
}
