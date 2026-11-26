import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t, useLanguage } from "../i18n";
import "./hoop.css";

interface BasketScorePayload {
  score: number;
  streak: number;
}

/**
 * Şeffaf masaüstü basketbol potası penceresi (§Mini Oyunlar, v0.3.0).
 */
export function BasketballHoopApp() {
  useLanguage();
  const [score, setScore] = useState(0);
  const [streak, setStreak] = useState(0);
  const [swishKey, setSwishKey] = useState(0);
  const [showToast, setShowToast] = useState(false);
  const toastTimerRef = useRef<number | null>(null);

  useEffect(() => {
    const unlistenScore = listen<BasketScorePayload>("basket-scored", (ev) => {
      setScore(ev.payload.score);
      setStreak(ev.payload.streak);
      setSwishKey((k) => k + 1);
      setShowToast(true);
      if (toastTimerRef.current !== null) {
        window.clearTimeout(toastTimerRef.current);
      }
      toastTimerRef.current = window.setTimeout(() => {
        setShowToast(false);
      }, 900);
    });

    const unlistenMode = listen<boolean>("basketball-mode-changed", (ev) => {
      if (!ev.payload) {
        setShowToast(false);
      }
    });

    return () => {
      if (toastTimerRef.current !== null) {
        window.clearTimeout(toastTimerRef.current);
      }
      void unlistenScore.then((f) => f());
      void unlistenMode.then((f) => f());
    };
  }, []);

  return (
    <div className={`hoop-stage ${swishKey > 0 ? "is-swish" : ""}`} key={swishKey}>
      <div className="hoop-hud">
        <span className="hoop-score-pill">
          <span>🏀 {score}</span>
          {streak > 1 && <span className="hoop-streak">×{streak}</span>}
        </span>
        <button
          type="button"
          className="hoop-close-btn"
          title={t("hoop.close")}
          aria-label={t("hoop.close")}
          onClick={() => void invoke("toggle_basketball_game")}
        >
          ✕
        </button>
      </div>

      {showToast && <div className="hoop-toast">{t("hoop.scored")}</div>}

      <svg viewBox="0 0 170 190" width="170" height="190" aria-hidden="true">
        {/* Şeffaf-buzlu panya */}
        <rect
          x="20"
          y="8"
          width="130"
          height="90"
          rx="10"
          fill="rgba(255,255,255,0.88)"
          stroke="#2E2A28"
          strokeWidth="3"
        />
        <rect
          x="55"
          y="38"
          width="60"
          height="42"
          rx="4"
          fill="none"
          stroke="#E03E2D"
          strokeWidth="3"
        />

        {/* File */}
        <g className="hoop-net">
          <path
            d="M40 92 L52 152 L118 152 L130 92"
            fill="rgba(255,255,255,0.22)"
            stroke="#FFFFFF"
            strokeWidth="2.4"
            strokeLinejoin="round"
          />
          <path
            d="M46 92 L72 152 M66 92 L95 152 M94 92 L65 152 M118 92 L92 152 M44 114 H126 M48 134 H122"
            stroke="#E5E7EB"
            strokeWidth="2"
            strokeLinecap="round"
          />
        </g>

        {/* Turuncu pota çemberi (pencere içi y ≈ 120'ye denk gelir) */}
        <rect x="77" y="84" width="16" height="9" rx="2" fill="#2E2A28" />
        <ellipse
          cx="85"
          cy="92"
          rx="51"
          ry="9"
          fill="none"
          stroke="#EA580C"
          strokeWidth="6.5"
        />
        <ellipse
          cx="85"
          cy="92"
          rx="51"
          ry="9"
          fill="none"
          stroke="#9A3412"
          strokeWidth="1.5"
        />
      </svg>
    </div>
  );
}
