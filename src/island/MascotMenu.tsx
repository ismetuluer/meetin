import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";

export interface MascotMenuProps {
  paused: boolean;
  soundEnabled: boolean;
  setSoundEnabled: Dispatch<SetStateAction<boolean>>;
  mascotSize: "small" | "normal" | "large";
  soundBaa: boolean;
  soundThrow: boolean;
  soundImpact: boolean;
  basketballMode: boolean;
  closeToMascot: () => Promise<void>;
}

// Sağ tık menüsü (bileşen değil: kipler arası DOM yeniden kullanımı korunur)
export function renderMascotMenu({
  paused,
  soundEnabled,
  setSoundEnabled,
  mascotSize,
  soundBaa,
  soundThrow,
  soundImpact,
  basketballMode,
  closeToMascot,
}: MascotMenuProps) {
  return (
    <div
      className="mascot-menu"
      onKeyDown={(e) => {
        if (e.key === "Escape") void closeToMascot();
      }}
    >
      <button
        type="button"
        className="mascot-menu-item"
        onClick={async () => {
          await invoke("toggle_pause");
          await closeToMascot();
        }}
      >
        {paused ? t("status.resume") : t("status.pause")}
      </button>
      <button
        type="button"
        className="mascot-menu-item"
        onClick={async () => {
          await invoke("open_studio_window");
          await closeToMascot();
        }}
      >
        {t("mascot.menuStudio")}
      </button>
      <button
        type="button"
        className="mascot-menu-item"
        onClick={async () => {
          const nextSound = !soundEnabled;
          setSoundEnabled(nextSound);
          await invoke("set_mascot_prefs", {
            sound: nextSound,
            size: mascotSize,
            soundBaa,
            soundThrow,
            soundImpact,
          });
          await closeToMascot();
        }}
      >
        {soundEnabled ? t("mascot.menuMute") : t("mascot.menuUnmute")}
      </button>
      <button
        type="button"
        className="mascot-menu-item"
        onClick={async () => {
          await invoke("toggle_basketball_game");
          await closeToMascot();
        }}
      >
        {basketballMode
          ? t("mascot.menuBasketStop")
          : t("mascot.menuBasketStart")}
      </button>
      <button
        type="button"
        className="mascot-menu-item"
        onClick={async () => {
          await closeToMascot();
          await invoke("toggle_mascot_visible");
        }}
      >
        {t("mascot.menuHide")}
      </button>
      <div className="mascot-menu-sep" />
      <button
        type="button"
        className="mascot-menu-item"
        onClick={() => void invoke("quit_app")}
      >
        {t("mascot.menuQuit")}
      </button>
    </div>
  );
}
