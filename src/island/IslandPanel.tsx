import {
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type RefObject,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { t } from "../i18n";
import { MeetinSheep } from "../mascot/MeetinSheep";
import type { SheepView } from "./islandTypes";

export interface PanelDrag {
  isDraggingPanel: boolean;
  handlePanelPointerDown: (e: ReactPointerEvent<HTMLDivElement>) => void;
  handlePanelPointerMove: (e: ReactPointerEvent<HTMLDivElement>) => void;
  handlePanelPointerUp: (e: ReactPointerEvent<HTMLDivElement>) => void;
}

// Arama / ekleme / doldurma adalarının üst çubuğundan serbestçe taşıma
export function usePanelDrag(isDraggingRef: RefObject<boolean>): PanelDrag {
  const [isDraggingPanel, setIsDraggingPanel] = useState(false);

  // Arama adası üst çubuk sürükleme durumu
  const panelDragRef = useRef<{
    down: boolean;
    pointerId: number | null;
    startScreenX: number;
    startScreenY: number;
    startWinX: number;
    startWinY: number;
    hasWinPos: boolean;
    lastSentX: number;
    lastSentY: number;
    dragStarted: boolean;
  }>({
    down: false,
    pointerId: null,
    startScreenX: 0,
    startScreenY: 0,
    startWinX: 0,
    startWinY: 0,
    hasWinPos: false,
    lastSentX: 0,
    lastSentY: 0,
    dragStarted: false,
  });

  function handlePanelPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest("input, button, select, textarea")) {
      return;
    }

    const pd = panelDragRef.current;
    pd.down = true;
    pd.pointerId = e.pointerId;
    pd.startScreenX = e.screenX;
    pd.startScreenY = e.screenY;
    pd.hasWinPos = false;
    pd.dragStarted = false;

    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Yok say
    }

    void getCurrentWebviewWindow()
      .outerPosition()
      .then((pos) => {
        pd.startWinX = pos.x;
        pd.startWinY = pos.y;
        pd.lastSentX = pos.x;
        pd.lastSentY = pos.y;
        pd.hasWinPos = true;
      })
      .catch(() => {});
  }

  function handlePanelPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const pd = panelDragRef.current;
    if (!pd.down || !pd.hasWinPos) return;
    const dx = e.screenX - pd.startScreenX;
    const dy = e.screenY - pd.startScreenY;
    if (!pd.dragStarted && Math.hypot(dx, dy) > 3) {
      pd.dragStarted = true;
      isDraggingRef.current = true;
      setIsDraggingPanel(true);
    }
    if (!pd.dragStarted) return;

    const dpr = window.devicePixelRatio || 1;
    const nextX = Math.round(pd.startWinX + dx * dpr);
    const nextY = Math.round(pd.startWinY + dy * dpr);
    if (nextX !== pd.lastSentX || nextY !== pd.lastSentY) {
      pd.lastSentX = nextX;
      pd.lastSentY = nextY;
      void invoke("move_mascot_window", { x: nextX, y: nextY });
    }
  }

  function handlePanelPointerUp(e: ReactPointerEvent<HTMLDivElement>) {
    const pd = panelDragRef.current;
    if (!pd.down) return;
    pd.down = false;
    if (pd.pointerId !== null) {
      try {
        if (e.currentTarget.hasPointerCapture(pd.pointerId)) {
          e.currentTarget.releasePointerCapture(pd.pointerId);
        }
      } catch {
        // Yok say
      }
    }
    pd.pointerId = null;

    if (pd.dragStarted) {
      pd.dragStarted = false;
      isDraggingRef.current = false;
      setIsDraggingPanel(false);
      if (pd.hasWinPos) {
        void invoke("save_mascot_position", {
          x: pd.lastSentX,
          y: pd.lastSentY,
        });
      }
    }
  }

  return {
    isDraggingPanel,
    handlePanelPointerDown,
    handlePanelPointerMove,
    handlePanelPointerUp,
  };
}

export interface PanelTopProps {
  drag: PanelDrag;
  sheep: SheepView;
  sheepSize: number;
  onClose: () => void;
}

// Üst çubuk: tutamak + kapatma koyunu + kipe özgü öğeler.
// Bileşen değil düz çizim işlevi: kipler arası geçişte React aynı DOM düğümlerini
// (koyun SVG'si dahil) bölünmeden önceki gibi yeniden kullanır.
export function renderPanelTop(
  { drag, sheep, sheepSize, onClose }: PanelTopProps,
  extra: ReactNode,
) {
  return (
    <div
      className={`island-search-top ${drag.isDraggingPanel ? "is-dragging" : ""}`}
      onPointerDown={drag.handlePanelPointerDown}
      onPointerMove={drag.handlePanelPointerMove}
      onPointerUp={drag.handlePanelPointerUp}
      onPointerCancel={drag.handlePanelPointerUp}
    >
      <span className="island-drag-grip" aria-hidden="true">
        ⋮⋮
      </span>
      <button
        type="button"
        className="island-mascot-close-btn"
        title={t("island.closeMascotTip")}
        aria-label={t("island.closeMascotTip")}
        onClick={onClose}
      >
        <MeetinSheep
          ref={sheep.svgRef}
          anim={sheep.anim}
          size={sheepSize}
          pupilOffset={sheep.pupilOffset}
          pausedMotion={!sheep.isVisible}
        />
      </button>
      {extra}
    </div>
  );
}
