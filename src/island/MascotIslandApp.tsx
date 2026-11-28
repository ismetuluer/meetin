import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reportCaughtError } from "../errorReporting";
import { t, useLanguage } from "../i18n";
import { MeetinSheep } from "../mascot/MeetinSheep";
import {
  ANIM_DURATIONS,
  computePupilOffset,
  pickAutoAnim,
  playBaaSound,
  type MascotAnim,
} from "../mascot/mascotMachine";
import type { FolderRecord, PhraseRecord, SuggestionItem } from "../studio/types";
import { renderAddIsland, useAddIsland } from "./AddIsland";
import { renderFillIsland, useFillIsland } from "./FillIsland";
import { usePanelDrag, type PanelTopProps } from "./IslandPanel";
import { renderNoticePill, renderSuggestBar } from "./IslandPills";
import type { IslandMode, PillNotice, SheepView } from "./islandTypes";
import { renderMascotMenu } from "./MascotMenu";
import { renderSearchIsland, useSearchIsland } from "./SearchIsland";
import { useIslandEvents } from "./useIslandEvents";
import { useMascotPointer } from "./useMascotPointer";
import { useThrowLoop } from "./useThrowLoop";
import "./island.css";

// Maskot penceresi: kip durumu burada; adalar, fırlatma ve olaylar ayrı dosyalarda.
// Kip çizimleri bileşen değil düz işlevdir; React, kipler arası geçişte DOM düğümlerini
// (koyun SVG'si dahil) tek dosyalı sürümdeki gibi yeniden kullanır.
export function MascotIslandApp() {
  useLanguage();
  const [mode, setMode] = useState<IslandMode>("mascot");
  const [anim, setAnim] = useState<MascotAnim>("idle");
  const [paused, setPaused] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const [isAdminWindow, setIsAdminWindow] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const [soundBaa, setSoundBaa] = useState(false);
  const [soundThrow, setSoundThrow] = useState(true);
  const [soundImpact, setSoundImpact] = useState(true);
  const [mascotSize, setMascotSize] = useState<"small" | "normal" | "large">("normal");
  const [pupilOffset, setPupilOffset] = useState({ x: 0, y: 0 });
  const [flightAngleDeg, setFlightAngleDeg] = useState(0);
  const [basketballMode, setBasketballMode] = useState(false);
  const [pillNotice, setPillNotice] = useState<PillNotice | null>(null);
  const [suggestions, setSuggestions] = useState<SuggestionItem[]>([]);
  const [updateAvailable, setUpdateAvailable] = useState<string | null>(null);
  const modeRef = useRef<IslandMode>("mascot");
  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  // Arama ve ekleme adalarının ortak verisi
  const [phrases, setPhrases] = useState<PhraseRecord[]>([]);
  const [folders, setFolders] = useState<FolderRecord[]>([]);

  const svgRef = useRef<SVGSVGElement | null>(null);
  const animTimerRef = useRef<number | null>(null);
  const pillTimerRef = useRef<number | null>(null);
  const lastUserRef = useRef<number>(Date.now());
  const isDraggingRef = useRef(false);
  const basketStatsRef = useRef({ score: 0, streak: 0 });
  const basketballModeRef = useRef(basketballMode);
  basketballModeRef.current = basketballMode;
  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  const soundEnabledRef = useRef(soundEnabled);
  soundEnabledRef.current = soundEnabled;
  const soundBaaRef = useRef(soundBaa);
  soundBaaRef.current = soundBaa;
  const soundThrowRef = useRef(soundThrow);
  soundThrowRef.current = soundThrow;
  const soundImpactRef = useRef(soundImpact);
  soundImpactRef.current = soundImpact;

  const clearAnimTimer = useCallback(() => {
    if (animTimerRef.current !== null) {
      window.clearTimeout(animTimerRef.current);
      animTimerRef.current = null;
    }
  }, []);

  const playAnim = useCallback(
    (next: MascotAnim, allowBaaSound = true) => {
      clearAnimTimer();
      setAnim(next);
      if (next === "baa" && allowBaaSound && soundEnabledRef.current && soundBaaRef.current) {
        playBaaSound();
      }
      const dur = ANIM_DURATIONS[next];
      if (dur !== null) {
        animTimerRef.current = window.setTimeout(() => {
          setAnim(pausedRef.current ? "sleep" : "idle");
        }, dur);
      }
    },
    [clearAnimTimer],
  );

  const showNoticePill = useCallback(
    (notice: PillNotice, nextAnim: MascotAnim) => {
      if (pillTimerRef.current !== null) {
        window.clearTimeout(pillTimerRef.current);
      }
      setPillNotice(notice);
      setMode("pill");
      playAnim(nextAnim);
      void invoke("set_mascot_mode", { mode: "pill" });

      pillTimerRef.current = window.setTimeout(() => {
        setMode("mascot");
        setPillNotice(null);
        void invoke("set_mascot_mode", { mode: "mascot" });
      }, 1200);
    },
    [playAnim],
  );

  const loadPhrases = useCallback(async () => {
    try {
      const [list, folderList] = await Promise.all([
        invoke<PhraseRecord[]>("list_phrases", { includeDeleted: false }),
        invoke<FolderRecord[]>("list_folders"),
      ]);
      setPhrases(list);
      setFolders(folderList);
    } catch (err) {
      reportCaughtError("load-phrases", err);
    }
  }, []);

  const { cancelThrow, startThrowLoop } = useThrowLoop({
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
  });

  const fill = useFillIsland({ cancelThrow, lastUserRef, setMode });
  const { fillPayload, setFillPayload } = fill;

  const closeToMascot = useCallback(async () => {
    if (mode === "fill") {
      await invoke("cancel_fill_island");
      setFillPayload(null);
    }
    setMode("mascot");
    await invoke("set_mascot_mode", { mode: "mascot" });
  }, [mode]);

  const search = useSearchIsland({
    phrases,
    loadPhrases,
    cancelThrow,
    lastUserRef,
    setMode,
    closeToMascot,
    playAnim,
    showNoticePill,
  });
  const { openSearch } = search;

  const add = useAddIsland({
    phrases,
    folders,
    setFolders,
    loadPhrases,
    cancelThrow,
    lastUserRef,
    setMode,
    closeToMascot,
    playAnim,
  });
  const { openAddIsland } = add;

  const pointer = useMascotPointer({
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
  });

  const panelDrag = usePanelDrag(isDraggingRef);

  useIslandEvents({
    modeRef,
    pillTimerRef,
    basketStatsRef,
    setMode,
    setAnim,
    setPaused,
    setIsAdminWindow,
    setIsVisible,
    setSoundEnabled,
    setSoundBaa,
    setSoundThrow,
    setSoundImpact,
    setMascotSize,
    setAbbrSuggestMode: add.setAbbrSuggestMode,
    setBasketballMode,
    setSuggestions,
    setUpdateAvailable,
    cancelThrow,
    clearAnimTimer,
    loadPhrases,
    playAnim,
    showNoticePill,
    openSearch,
    openAddIsland,
    openFillIsland: fill.openFillIsland,
  });

  // "Kendi hâlinde" modu: 5 sn etkileşim yoksa 6 sn'de bir rastgele animasyon (§5.1.1)
  useEffect(() => {
    const id = window.setInterval(() => {
      if (
        !isVisible ||
        paused ||
        mode !== "mascot" ||
        anim !== "idle" ||
        Date.now() - lastUserRef.current < 5000
      ) {
        return;
      }
      playAnim(pickAutoAnim(), false);
    }, 6000);
    return () => window.clearInterval(id);
  }, [anim, isVisible, mode, paused, playAnim]);

  // Göz bebeği takibi (§5.1.1)
  useEffect(() => {
    function onPointerMove(e: PointerEvent) {
      if (paused || anim === "sleep" || !isVisible || !svgRef.current) return;
      const rect = svgRef.current.getBoundingClientRect();
      const cx = rect.left + rect.width * 0.5;
      const cy = rect.top + rect.height * 0.53;
      setPupilOffset(computePupilOffset(e.clientX - cx, e.clientY - cy));
    }
    window.addEventListener("pointermove", onPointerMove);
    return () => window.removeEventListener("pointermove", onPointerMove);
  }, [anim, isVisible, paused]);

  // Ada/menü açıkken odak nerede olursa olsun Esc ile kapat + odak kaybında kapat
  useEffect(() => {
    if (mode !== "search" && mode !== "add" && mode !== "fill" && mode !== "menu") return;

    function onWindowKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void closeToMascot();
      }
    }
    function onWindowBlur() {
      if (isDraggingRef.current) return;
      void closeToMascot();
    }

    window.addEventListener("keydown", onWindowKeyDown);
    window.addEventListener("blur", onWindowBlur);
    return () => {
      window.removeEventListener("keydown", onWindowKeyDown);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [closeToMascot, mode]);

  const sheep: SheepView = { svgRef, anim, pupilOffset, isVisible };
  const panelTop = (sheepSize: number): PanelTopProps => ({
    drag: panelDrag,
    sheep,
    sheepSize,
    onClose: () => void closeToMascot(),
  });

  if (mode === "menu") {
    return renderMascotMenu({
      paused,
      soundEnabled,
      setSoundEnabled,
      mascotSize,
      soundBaa,
      soundThrow,
      soundImpact,
      basketballMode,
      closeToMascot,
    });
  }

  if (mode === "suggest" && suggestions.length > 0) {
    return renderSuggestBar(sheep, suggestions, updateAvailable);
  }

  if (mode === "pill" && pillNotice) {
    return renderNoticePill(sheep, pillNotice);
  }

  if (mode === "search") {
    return renderSearchIsland(panelTop(48), search);
  }

  if (mode === "add") {
    return renderAddIsland(panelTop(44), add);
  }

  if (mode === "fill" && fillPayload) {
    return renderFillIsland(panelTop(44), fill, fillPayload, setMode, closeToMascot);
  }

  return (
    <div className="mascot-stage">
      <button
        type="button"
        className="mascot-hit"
        aria-label={t("mascot.ariaLabel")}
        onPointerDown={pointer.handleMascotPointerDown}
        onPointerMove={pointer.handleMascotPointerMove}
        onPointerUp={pointer.handleMascotPointerUp}
        onPointerCancel={pointer.handleMascotPointerCancel}
        onLostPointerCapture={pointer.handleMascotPointerCancel}
        onContextMenu={pointer.handleMascotContextMenu}
      >
        <MeetinSheep
          ref={svgRef}
          anim={paused ? "sleep" : anim}
          size={mascotSize === "small" ? 62 : mascotSize === "large" ? 84 : 70}
          pupilOffset={pupilOffset}
          flightAngleDeg={flightAngleDeg}
          pausedMotion={!isVisible}
          showAdminBadge={isAdminWindow}
          adminTooltip={t("mascot.adminTooltip")}
          showUpdateDot={Boolean(updateAvailable)}
        />
      </button>
    </div>
  );
}
