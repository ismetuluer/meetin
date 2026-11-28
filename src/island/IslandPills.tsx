import { invoke } from "@tauri-apps/api/core";
import { MeetinSheep } from "../mascot/MeetinSheep";
import type { SuggestionItem } from "../studio/types";
import type { PillNotice, SheepView } from "./islandTypes";

// Yazarken öneri hapları (bileşen değil: kipler arası DOM yeniden kullanımı korunur)
export function renderSuggestBar(
  sheep: SheepView,
  suggestions: SuggestionItem[],
  updateAvailable: string | null,
) {
  return (
    <div className="mascot-stage">
      <div className="island-suggest-bar">
        <MeetinSheep
          ref={sheep.svgRef}
          anim="stare"
          size={42}
          pupilOffset={sheep.pupilOffset}
          pausedMotion={!sheep.isVisible}
          showUpdateDot={Boolean(updateAvailable)}
        />
        <div className="suggest-pills-row">
          {suggestions.map((item, idx) => (
            <button
              key={item.phrase_id}
              type="button"
              className={`suggest-pill ${idx === 0 ? "is-primary" : ""}`}
              onClick={() =>
                void invoke("accept_typing_suggestion", {
                  phraseId: item.phrase_id,
                })
              }
              title={item.preview}
            >
              {idx === 0 && <span className="suggest-kbd">Tab</span>}
              <strong>{item.abbr}</strong>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

// Kısa bildirim hapı (eklendi / hata)
export function renderNoticePill(sheep: SheepView, pillNotice: PillNotice) {
  return (
    <div className="mascot-stage">
      <div className="island-pill">
        <MeetinSheep
          ref={sheep.svgRef}
          anim={sheep.anim}
          size={48}
          pupilOffset={sheep.pupilOffset}
          pausedMotion={!sheep.isVisible}
        />
        <span
          className={`island-pill-text ${pillNotice.ok ? "island-pill-ok" : "island-pill-err"}`}
        >
          {pillNotice.text}
        </span>
      </div>
    </div>
  );
}
