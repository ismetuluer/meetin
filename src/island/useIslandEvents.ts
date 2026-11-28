import { useEffect, type Dispatch, type RefObject, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { reportCaughtError } from "../errorReporting";
import { t } from "../i18n";
import type { MascotAnim } from "../mascot/mascotMachine";
import type {
  AbbrSuggestMode,
  AppSettingsDto,
  EngineStatus,
  SuggestionItem,
} from "../studio/types";
import type { FillIslandPayload, IslandMode, PillNotice } from "./islandTypes";

type Setter<T> = Dispatch<SetStateAction<T>>;

interface IslandEventDeps {
  modeRef: RefObject<IslandMode>;
  pillTimerRef: RefObject<number | null>;
  basketStatsRef: RefObject<{ score: number; streak: number }>;
  setMode: Setter<IslandMode>;
  setAnim: Setter<MascotAnim>;
  setPaused: Setter<boolean>;
  setIsAdminWindow: Setter<boolean>;
  setIsVisible: Setter<boolean>;
  setSoundEnabled: Setter<boolean>;
  setSoundBaa: Setter<boolean>;
  setSoundThrow: Setter<boolean>;
  setSoundImpact: Setter<boolean>;
  setMascotSize: Setter<"small" | "normal" | "large">;
  setAbbrSuggestMode: Setter<AbbrSuggestMode>;
  setBasketballMode: Setter<boolean>;
  setSuggestions: Setter<SuggestionItem[]>;
  setUpdateAvailable: Setter<string | null>;
  cancelThrow: () => void;
  clearAnimTimer: () => void;
  loadPhrases: () => Promise<void>;
  playAnim: (next: MascotAnim, allowBaaSound?: boolean) => void;
  showNoticePill: (notice: PillNotice, nextAnim: MascotAnim) => void;
  openSearch: () => Promise<void>;
  openAddIsland: (initialText: string) => Promise<void>;
  openFillIsland: (payload: FillIslandPayload) => void;
}

// İlk durum yükleme ve Tauri olay dinleyicileri
export function useIslandEvents({
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
  setAbbrSuggestMode,
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
  openFillIsland,
}: IslandEventDeps) {
  useEffect(() => {
    void invoke<EngineStatus>("get_engine_status").then((st) => {
      setPaused(st.paused);
      if (st.paused) setAnim("sleep");
    });
    void invoke<AppSettingsDto>("get_app_settings")
      .then((s) => {
        setSoundEnabled(s.mascot_sound);
        setSoundBaa(s.sound_baa ?? false);
        setSoundThrow(s.sound_throw ?? true);
        setSoundImpact(s.sound_impact ?? true);
        setMascotSize(s.mascot_size);
        setAbbrSuggestMode(s.abbr_suggest_mode ?? "initials");
      })
      .catch((err) => reportCaughtError("load-settings", err));
    void loadPhrases();

    const unlisteners = [
      listen<{
        sound: boolean;
        size: "small" | "normal" | "large";
        sound_baa?: boolean;
        sound_throw?: boolean;
        sound_impact?: boolean;
      }>("mascot-prefs-changed", (ev) => {
        setSoundEnabled(ev.payload.sound);
        setMascotSize(ev.payload.size);
        if (typeof ev.payload.sound_baa === "boolean") {
          setSoundBaa(ev.payload.sound_baa);
        }
        if (typeof ev.payload.sound_throw === "boolean") {
          setSoundThrow(ev.payload.sound_throw);
        }
        if (typeof ev.payload.sound_impact === "boolean") {
          setSoundImpact(ev.payload.sound_impact);
        }
      }),
      listen<void>("onboarding-wake-mascot", () => {
        playAnim("hop");
      }),
      listen<string>("phrase-expanded", () => {
        playAnim("thumbsup");
      }),
      listen<string>("phrase-created", () => {
        playAnim("thumbsup");
      }),
      listen<string>("engine-error", (ev) => {
        showNoticePill(
          { ok: false, text: ev.payload || t("island.notFound") },
          "sad",
        );
      }),
      listen<boolean>("engine-status-changed", (ev) => {
        setPaused(ev.payload);
        setAnim(ev.payload ? "sleep" : "idle");
      }),
      listen<boolean>("admin-window-changed", (ev) => {
        setIsAdminWindow(ev.payload);
      }),
      listen<boolean>("mascot-visibility-changed", (ev) => {
        setIsVisible(ev.payload);
      }),
      listen<void>("open-search-island", () => {
        void openSearch();
      }),
      listen<string>("open-add-island", (ev) => {
        void openAddIsland(ev.payload ?? "");
      }),
      listen<FillIslandPayload>("open-fill-island", (ev) => {
        openFillIsland(ev.payload);
      }),
      listen<number>("file-imported", (ev) => {
        showNoticePill(
          { ok: true, text: t("island.fileImportedOk", { count: ev.payload }) },
          "thumbsup",
        );
      }),
      listen<boolean>("basketball-mode-changed", (ev) => {
        setBasketballMode(ev.payload);
        if (!ev.payload) {
          basketStatsRef.current = { score: 0, streak: 0 };
        }
      }),
      listen<SuggestionItem[]>("typing-suggestions", (ev) => {
        const items = ev.payload ?? [];
        const curMode = modeRef.current;
        if (items.length > 0 && (curMode === "mascot" || curMode === "suggest")) {
          setSuggestions(items);
          if (curMode !== "suggest") {
            setMode("suggest");
            void invoke("set_mascot_mode", { mode: "suggest" });
          }
        } else if (items.length === 0 && curMode === "suggest") {
          setSuggestions([]);
          setMode("mascot");
          void invoke("set_mascot_mode", { mode: "mascot" });
        }
      }),
      listen<string>("update-available", (ev) => {
        setUpdateAvailable(ev.payload || "yeni");
      }),
    ];
    // Açılış denetimi dinleyici bağlanmadan bitmiş olabilir: sonucu ayrıca sor.
    void Promise.all(unlisteners)
      .then(() =>
        invoke<{
          update_available: boolean;
          latest_version: string | null;
        }>("get_update_env_info"),
      )
      .then((u) => {
        if (u.update_available) {
          setUpdateAvailable(u.latest_version || "yeni");
        }
      })
      .catch((err) => console.error("get_update_env_info", err));

    return () => {
      cancelThrow();
      clearAnimTimer();
      if (pillTimerRef.current !== null) {
        window.clearTimeout(pillTimerRef.current);
      }
      for (const u of unlisteners) {
        void u.then((f) => f());
      }
    };
  }, [clearAnimTimer, loadPhrases, openAddIsland, openSearch, showNoticePill]);
}
