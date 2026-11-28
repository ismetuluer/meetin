import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { AppRule, RunningAppInfo } from "./types";

interface StudioRulesProps {
  draftId: number | null;
  appRules: AppRule[];
  setAppRules: Dispatch<SetStateAction<AppRule[]>>;
  runningApps: RunningAppInfo[];
  newRuleProc: string;
  setNewRuleProc: Dispatch<SetStateAction<string>>;
  newRuleKind: AppRule["kind"];
  setNewRuleKind: Dispatch<SetStateAction<AppRule["kind"]>>;
  showRuleAdd: boolean;
  setShowRuleAdd: Dispatch<SetStateAction<boolean>>;
  refreshRunningApps: () => Promise<void>;
  setError: Dispatch<SetStateAction<string | null>>;
}

/** Uygulama kuralları: ifade yalnız / hariç tutulan uygulamalarda çalışır. */
export function StudioRules({
  draftId,
  appRules,
  setAppRules,
  runningApps,
  newRuleProc,
  setNewRuleProc,
  newRuleKind,
  setNewRuleKind,
  showRuleAdd,
  setShowRuleAdd,
  refreshRunningApps,
  setError,
}: StudioRulesProps) {
  async function persistRules(nextRules: AppRule[]) {
    setAppRules(nextRules);
    if (draftId !== null) {
      try {
        await invoke("set_phrase_app_rules", {
          phraseId: draftId,
          rules: nextRules,
        });
      } catch (err) {
        setError(t("error.generic", { message: String(err) }));
      }
    }
  }

  return (
    <div className="rules-inline-section">
      <span className="tiny-label">{t("studio.tabRules")}</span>
      <div className="rules-chip-row">
        {appRules.length === 0 && (
          <span className="muted small">{t("studio.rulesAllApps")}</span>
        )}
        {appRules.map((r, idx) => (
          <span key={idx} className={`tag-pill rule-pill is-${r.kind}`}>
            <span className="rule-kind">
              {r.kind === "exclude" ? t("studio.ruleKindExclude") : t("studio.ruleKindOnly")}
            </span>
            {r.process_name}
            <button
              type="button"
              onClick={() =>
                void persistRules(
                  appRules.filter((_, i) => i !== idx),
                )
              }
            >
              ×
            </button>
          </span>
        ))}
        <button
          type="button"
          className="link-btn"
          title={t("tip.addRule")}
          onClick={() => {
            const next = !showRuleAdd;
            setShowRuleAdd(next);
            if (next) {
              void refreshRunningApps();
            }
          }}
        >
          {t("studio.addRule")}
        </button>
      </div>

      {showRuleAdd && (
        <div className="inline-rule-panel">
          <div className="inline-rule-editor">
            <select
              className="select"
              style={{ width: 180 }}
              value={newRuleKind}
              onChange={(e) =>
                setNewRuleKind(e.target.value as AppRule["kind"])
              }
            >
              <option value="only">{t("studio.ruleOnly")}</option>
              <option value="exclude">{t("studio.ruleExclude")}</option>
            </select>
            <input
              className="input"
              value={newRuleProc}
              onChange={(e) => setNewRuleProc(e.target.value)}
              placeholder={t("studio.customProcess")}
            />
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => {
                const proc = newRuleProc.trim();
                if (!proc) return;
                void persistRules([
                  ...appRules,
                  {
                    kind: newRuleKind,
                    process_name: proc,
                    title_contains: null,
                  },
                ]);
                setNewRuleProc("");
                setShowRuleAdd(false);
              }}
            >
              {t("form.save")}
            </button>
          </div>
          <div className="running-apps-picker">
            <div className="running-apps-head">
              <span className="muted small">{t("studio.detectApps")}</span>
              <button
                type="button"
                className="link-btn"
                onClick={() => void refreshRunningApps()}
              >
                {t("studio.refreshApps")}
              </button>
            </div>
            {runningApps.length === 0 ? (
              <span className="muted small">{t("studio.detectAppsEmpty")}</span>
            ) : (
              <div className="running-apps-chips">
                {runningApps.map((app) => (
                  <button
                    key={app.process_name}
                    type="button"
                    className="btn btn-xs"
                    title={app.window_title}
                    onClick={() => {
                      const already = appRules.some(
                        (r) =>
                          r.kind === newRuleKind &&
                          r.process_name.toLowerCase() ===
                            app.process_name.toLowerCase(),
                      );
                      if (!already) {
                        void persistRules([
                          ...appRules,
                          {
                            kind: newRuleKind,
                            process_name: app.process_name,
                            title_contains: null,
                          },
                        ]);
                      }
                      setShowRuleAdd(false);
                    }}
                  >
                    + {app.process_name}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
