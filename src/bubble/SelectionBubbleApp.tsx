import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import "./bubble.css";

/**
 * Metin seçilince imlecin yanında beliren "+ Ekle" kabarcığı (§4.1, M3.2).
 */
export function SelectionBubbleApp() {
  return (
    <div className="bubble-stage">
      <button
        type="button"
        className="selection-bubble-btn"
        onClick={() => void invoke("accept_selection_bubble")}
      >
        {t("bubble.addBtn")}
      </button>
    </div>
  );
}
