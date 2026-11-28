import {
  useCallback,
  useRef,
  useState,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { renderPanelTop, type PanelTopProps } from "./IslandPanel";
import type { FillIslandPayload, IslandMode } from "./islandTypes";

interface FillIslandDeps {
  cancelThrow: () => void;
  lastUserRef: RefObject<number>;
  setMode: Dispatch<SetStateAction<IslandMode>>;
}

// Alan doldurma adası durumu (§3.3, §5.2)
export function useFillIsland({ cancelThrow, lastUserRef, setMode }: FillIslandDeps) {
  const [fillPayload, setFillPayload] = useState<FillIslandPayload | null>(null);
  const [fillValues, setFillValues] = useState<Record<string, string>>({});
  const fillFirstInputRef = useRef<HTMLInputElement | HTMLSelectElement | null>(null);

  // "open-fill-island" olayı: alanları ilk seçenekle (ya da boş) doldurup adayı açar
  const openFillIsland = useCallback(
    (payload: FillIslandPayload) => {
      cancelThrow();
      lastUserRef.current = Date.now();
      const initialVals: Record<string, string> = {};
      for (const f of payload.fields) {
        initialVals[f.name] = f.options?.[0] ?? "";
      }
      setFillPayload(payload);
      setFillValues(initialVals);
      setMode("fill");
      window.setTimeout(() => fillFirstInputRef.current?.focus(), 45);
    },
    [cancelThrow, lastUserRef, setMode],
  );

  return {
    fillPayload,
    setFillPayload,
    fillValues,
    setFillValues,
    fillFirstInputRef,
    openFillIsland,
  };
}

export type FillIslandState = ReturnType<typeof useFillIsland>;

// Alan doldurma adası çizimi (bileşen değil: kipler arası DOM yeniden kullanımı korunur)
export function renderFillIsland(
  top: PanelTopProps,
  fill: FillIslandState,
  fillPayload: FillIslandPayload,
  setMode: Dispatch<SetStateAction<IslandMode>>,
  closeToMascot: () => Promise<void>,
) {
  const { fillValues, setFillValues, fillFirstInputRef } = fill;
  return (
    <div className="island-search-panel">
      {renderPanelTop(
        top,
        <>
          <span className="island-add-title">
            {t("fill.title", { abbr: fillPayload.abbr })}
          </span>
          <button
            type="button"
            className="island-kbd"
            onClick={top.onClose}
          >
            Esc
          </button>
        </>,
      )}

      <form
        className="island-add-body"
        onSubmit={async (e) => {
          e.preventDefault();
          setMode("mascot");
          try {
            await invoke("submit_fill_island", { values: fillValues });
            fill.setFillPayload(null);
          } catch {
            await closeToMascot();
          }
        }}
      >
        {fillPayload.fields.map((field, idx) => (
          <label key={field.name} className="island-field">
            <span>{field.name}</span>
            {field.options ? (
              <select
                ref={
                  idx === 0
                    ? (el) => {
                        fillFirstInputRef.current = el;
                      }
                    : undefined
                }
                className="select"
                value={fillValues[field.name] ?? ""}
                onChange={(e) =>
                  setFillValues({
                    ...fillValues,
                    [field.name]: e.target.value,
                  })
                }
              >
                {field.options.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
            ) : (
              <input
                ref={
                  idx === 0
                    ? (el) => {
                        fillFirstInputRef.current = el;
                      }
                    : undefined
                }
                className="input"
                value={fillValues[field.name] ?? ""}
                onChange={(e) =>
                  setFillValues({
                    ...fillValues,
                    [field.name]: e.target.value,
                  })
                }
              />
            )}
          </label>
        ))}

        <div className="island-add-actions">
          <span className="muted small">{t("fill.hint")}</span>
          <div className="island-add-actions-right">
            <button
              type="button"
              className="btn island-mini-btn"
              onClick={top.onClose}
            >
              {t("form.cancel")}
            </button>
            <button type="submit" className="btn btn-primary island-mini-btn">
              {t("fill.insert")}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
