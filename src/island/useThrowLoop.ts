import { useCallback, useRef, type Dispatch, type RefObject, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reportCaughtError } from "../errorReporting";
import {
  playRocketSound,
  playThrowSound,
  playWallImpactSound,
  type MascotAnim,
} from "../mascot/mascotMachine";
import {
  checkBasketScore,
  hostMonitorIndex,
  predictCrossesMonitor,
  stepThrowPhysicsMulti,
  windowSizeOn,
  type HoopRimRect,
  type MonitorLayout,
  type MonitorRect,
  type ThrowState,
} from "../mascot/throwPhysics";

interface ThrowLoopDeps {
  clearAnimTimer: () => void;
  setAnim: Dispatch<SetStateAction<MascotAnim>>;
  setFlightAngleDeg: Dispatch<SetStateAction<number>>;
  playAnim: (next: MascotAnim, allowBaaSound?: boolean) => void;
  basketballModeRef: RefObject<boolean>;
  basketStatsRef: RefObject<{ score: number; streak: number }>;
  pausedRef: RefObject<boolean>;
  soundEnabledRef: RefObject<boolean>;
  soundThrowRef: RefObject<boolean>;
  soundImpactRef: RefObject<boolean>;
}

// Fırlatma döngüsü: çoklu monitör fiziği, roket seçimi, duvar sesi ve basket sayımı (R3.0f)
export function useThrowLoop({
  clearAnimTimer,
  setAnim,
  setFlightAngleDeg,
  playAnim,
  basketballModeRef,
  basketStatsRef,
  pausedRef,
  soundEnabledRef,
  soundThrowRef,
  soundImpactRef,
}: ThrowLoopDeps) {
  const throwRafRef = useRef<number | null>(null);
  const lastImpactSoundRef = useRef(0);

  const cancelThrow = useCallback(() => {
    if (throwRafRef.current !== null) {
      window.cancelAnimationFrame(throwRafRef.current);
      throwRafRef.current = null;
    }
  }, []);

  function startThrowLoop(initial: ThrowState) {
    cancelThrow();
    const isBasket = basketballModeRef.current;
    const gravity = isBasket ? 1150 : 0;

    clearAnimTimer();
    setAnim("super");
    setFlightAngleDeg((Math.atan2(initial.vy, initial.vx) * 180) / Math.PI);

    void Promise.all([
      invoke<{
        monitors: MonitorRect[];
        window_width: number;
        window_height: number;
      }>("get_mascot_monitor_layout"),
      isBasket
        ? invoke<HoopRimRect | null>("get_hoop_rim_rect")
        : Promise.resolve(null),
    ])
      .then(([dto, initialRim]) => {
        const layout: MonitorLayout = {
          monitors: dto.monitors,
          windowWidth: dto.window_width,
          windowHeight: dto.window_height,
        };
        // Atış başka monitöre ulaşacaksa sırtta roket (R3.0f); azaltılmış harekette yok
        const rocket =
          !window.matchMedia("(prefers-reduced-motion: reduce)").matches &&
          predictCrossesMonitor(initial, layout, { gravity });
        if (rocket) setAnim("rocket");
        if (soundEnabledRef.current && soundThrowRef.current) {
          if (rocket) playRocketSound();
          else playThrowSound();
        }
        const rim = initialRim;
        let host = hostMonitorIndex(initial.x, initial.y, layout);
        let sim = initial;
        let lastTime = performance.now();
        let lastSavedX = Math.round(sim.x);
        let lastSavedY = Math.round(sim.y);
        let scoredThisThrow = false;

        const tick = (now: number) => {
          const dtSec = (now - lastTime) / 1000;
          lastTime = now;

          const half = windowSizeOn(layout.monitors[host], layout).w / 2;
          const prevCx = sim.x + half;
          const prevCy = sim.y + half;
          const step = stepThrowPhysicsMulti(sim, layout, dtSec, { gravity });
          sim = step.state;
          host = step.monitor;
          if (step.bounced && soundEnabledRef.current && soundImpactRef.current) {
            const sinceLast = now - lastImpactSoundRef.current;
            if (sinceLast >= 90) {
              lastImpactSoundRef.current = now;
              playWallImpactSound(step.impactSpeed);
            }
          }
          const nextCx = sim.x + half;
          const nextCy = sim.y + half;

          const rx = Math.round(sim.x);
          const ry = Math.round(sim.y);
          if (rx !== lastSavedX || ry !== lastSavedY) {
            lastSavedX = rx;
            lastSavedY = ry;
            void invoke("move_mascot_window", { x: rx, y: ry });
          }

          if (!scoredThisThrow) {
            setFlightAngleDeg((Math.atan2(sim.vy, sim.vx) * 180) / Math.PI);
          }

          if (
            isBasket &&
            rim &&
            !scoredThisThrow &&
            checkBasketScore(prevCx, prevCy, nextCx, nextCy, rim)
          ) {
            scoredThisThrow = true;
            basketStatsRef.current.score += 1;
            basketStatsRef.current.streak += 1;
            const { score, streak } = basketStatsRef.current;
            void invoke("notify_basket_scored", { score, streak });
            playAnim("thumbsup");
            window.setTimeout(() => {
              void invoke("reposition_hoop_random");
            }, 850);
          }

          if (step.stopped) {
            throwRafRef.current = null;
            if (isBasket && !scoredThisThrow) {
              basketStatsRef.current.streak = 0;
            }
            if (!scoredThisThrow) {
              setAnim(pausedRef.current ? "sleep" : "idle");
            }
            void invoke("save_mascot_position", {
              x: lastSavedX,
              y: lastSavedY,
            });
            return;
          }

          throwRafRef.current = window.requestAnimationFrame(tick);
        };

        throwRafRef.current = window.requestAnimationFrame(tick);
      })
      .catch((err: unknown) => {
        // Monitör düzeni alınamadı: atış yapılmaz, konum korunur, neden günlüğe yazılır
        reportCaughtError("throw", err);
        setAnim(pausedRef.current ? "sleep" : "idle");
        void invoke("save_mascot_position", {
          x: Math.round(initial.x),
          y: Math.round(initial.y),
        });
      });
  }

  return { cancelThrow, startThrowLoop };
}
