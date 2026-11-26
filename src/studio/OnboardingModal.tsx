import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { MeetinSheep } from "../mascot/MeetinSheep";

interface OnboardingModalProps {
  initialName: string;
  onFinish: () => void;
}

export function OnboardingModal({
  initialName,
  onFinish,
}: OnboardingModalProps) {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [awake, setAwake] = useState(false);
  const [userName, setUserName] = useState(initialName);
  const [busy, setBusy] = useState(false);

  const handleWake = () => {
    setAwake(true);
  };

  const handleComplete = async () => {
    setBusy(true);
    try {
      await invoke("complete_onboarding", { userName: userName.trim() });
      onFinish();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-card onboarding-card">
        <div className="modal-header">
          <h2>{t("onboarding.title")}</h2>
          <span className="muted small">{step} / 3</span>
        </div>

        <div className="modal-body">
          {step === 1 && (
            <div className="onboarding-hero">
              <div
                onClick={handleWake}
                style={{ cursor: "pointer" }}
                title={t("onboarding.wakeBtn")}
              >
                <MeetinSheep anim={awake ? "hop" : "sleep"} size={92} />
              </div>
              <strong>
                {awake
                  ? t("onboarding.awakeTitle")
                  : t("onboarding.sleepTitle")}
              </strong>
              <p className="muted small">{t("onboarding.step1Desc")}</p>
              {!awake && (
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={handleWake}
                  title={t("onboarding.wakeBtn")}
                >
                  {t("onboarding.wakeBtn")}
                </button>
              )}
              <label style={{ width: "100%", textAlign: "left", marginTop: 6 }}>
                <span className="tiny-label">{t("onboarding.nameLabel")}</span>
                <input
                  className="input"
                  value={userName}
                  onChange={(e) => setUserName(e.target.value)}
                  placeholder={t("onboarding.namePlaceholder")}
                  title={t("onboarding.nameLabel")}
                />
              </label>
            </div>
          )}

          {step === 2 && (
            <div className="onboarding-features">
              <div className="onboarding-feature-box">
                <strong>1. {t("onboarding.feat1Title")}</strong>
                <span className="muted small">{t("onboarding.feat1Desc")}</span>
              </div>
              <div className="onboarding-feature-box">
                <strong>2. {t("onboarding.feat2Title")}</strong>
                <span className="muted small">{t("onboarding.feat2Desc")}</span>
              </div>
              <div className="onboarding-feature-box">
                <strong>3. {t("onboarding.feat3Title")}</strong>
                <span className="muted small">{t("onboarding.feat3Desc")}</span>
              </div>
            </div>
          )}

          {step === 3 && (
            <div className="onboarding-hero">
              <MeetinSheep anim="thumbsup" size={76} />
              <strong>
                {userName.trim()
                  ? t("onboarding.readyNamed", { name: userName.trim() })
                  : t("onboarding.readyAnon")}
              </strong>
              <p className="muted small">{t("onboarding.readyHint")}</p>
            </div>
          )}
        </div>

        <div className="modal-footer">
          {step > 1 && (
            <button
              type="button"
              className="btn"
              onClick={() => setStep((s) => (s === 3 ? 2 : 1))}
              title={t("onboarding.back")}
            >
              {t("onboarding.back")}
            </button>
          )}
          {step < 3 ? (
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                handleWake();
                setStep((s) => (s === 1 ? 2 : 3));
              }}
              title={t("onboarding.next")}
            >
              {t("onboarding.next")}
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void handleComplete()}
              title={t("onboarding.start")}
            >
              {t("onboarding.start")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
