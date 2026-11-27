import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  setLanguage,
  t,
  useLanguage,
  type Language,
  type TranslationKey,
} from "../i18n";
import {
  playBaaSound,
  playThrowSound,
  playWallImpactSound,
} from "../mascot/mascotMachine";
import { AppearanceSettings } from "./AppearanceSettings";
import { captureShortcutCombo } from "./shortcutCapture";
import type {
  AbbrSuggestMode,
  AppSettingsDto,
  BackupFileInfo,
  EngineStatus,
  RunningAppInfo,
  ShortcutConfigDto,
  StartupStatus,
  UpdateCheckResult,
  UpdateInstallResult,
} from "./types";
import "./settings.css";

type SettingsSection =
  | "general"
  | "trigger"
  | "case"
  | "appearance"
  | "mascot"
  | "shortcuts"
  | "excluded"
  | "backup"
  | "update"
  | "language";

interface SettingsViewProps {
  status: EngineStatus | null;
  initialSection?: SettingsSection;
  onRefresh: () => void;
  onExport: (format: "meetin" | "csv" | "ahk") => Promise<void>;
  onReplayOnboarding: () => void;
}

const DEFAULT_EXCLUDED: string[] = [];

const DEFAULT_SHORTCUTS: ShortcutConfigDto = {
  pause: "Ctrl+Alt+P",
  search: "Ctrl+Alt+F",
  quick_add: "Ctrl+Alt+E",
  studio: "Ctrl+Alt+S",
  toggle_mascot: "Ctrl+Alt+Space",
};

const NAV_ITEMS: { id: SettingsSection; labelKey: TranslationKey }[] = [
  { id: "general", labelKey: "settings.nav.general" },
  { id: "trigger", labelKey: "settings.nav.trigger" },
  { id: "case", labelKey: "settings.nav.case" },
  { id: "appearance", labelKey: "settings.nav.appearance" },
  { id: "mascot", labelKey: "settings.nav.mascot" },
  { id: "shortcuts", labelKey: "settings.nav.shortcuts" },
  { id: "excluded", labelKey: "settings.nav.excluded" },
  { id: "backup", labelKey: "settings.nav.backup" },
  { id: "update", labelKey: "settings.nav.update" },
  { id: "language", labelKey: "settings.nav.language" },
];

export function SettingsView({
  status,
  initialSection,
  onRefresh,
  onExport,
  onReplayOnboarding,
}: SettingsViewProps) {
  const activeLanguage = useLanguage();
  const [section, setSection] = useState<SettingsSection>(
    initialSection ?? "appearance",
  );

  useEffect(() => {
    if (initialSection) {
      setSection(initialSection);
    }
  }, [initialSection]);
  const [settings, setSettings] = useState<AppSettingsDto | null>(null);
  const [userNameDraft, setUserNameDraft] = useState("");
  const [shortcutDraft, setShortcutDraft] =
    useState<ShortcutConfigDto>(DEFAULT_SHORTCUTS);
  const [newProc, setNewProc] = useState("");
  const [runningApps, setRunningApps] = useState<RunningAppInfo[]>([]);
  const [backups, setBackups] = useState<BackupFileInfo[]>([]);
  const [startup, setStartup] = useState<StartupStatus | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResult | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [confirmRestore, setConfirmRestore] = useState<string | null>(null);
  const [busyMsg, setBusyMsg] = useState<string | null>(null);
  const [errMsg, setErrMsg] = useState<string | null>(null);

  const loadSettings = useCallback(async () => {
    try {
      const [s, list, stp, upd] = await Promise.all([
        invoke<AppSettingsDto>("get_app_settings"),
        invoke<BackupFileInfo[]>("list_backups"),
        invoke<StartupStatus>("get_startup_status"),
        invoke<UpdateCheckResult>("get_update_env_info"),
      ]);
      setSettings(s);
      setUserNameDraft(s.user_name);
      setShortcutDraft(s.shortcuts);
      setBackups(list);
      setStartup(stp);
      setUpdateInfo(upd);
    } catch (e) {
      setErrMsg(String(e));
    }
  }, []);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  const shortcutConflict = useMemo(() => {
    const vals = [
      shortcutDraft.pause,
      shortcutDraft.search,
      shortcutDraft.quick_add,
      shortcutDraft.studio,
      shortcutDraft.toggle_mascot,
    ].map((v) => v.trim().toLowerCase());
    return new Set(vals.filter(Boolean)).size !== vals.length;
  }, [shortcutDraft]);

  const togglePause = async () => {
    await invoke("toggle_pause");
    onRefresh();
  };

  const toggleFoldI = async (enabled: boolean) => {
    await invoke("set_fold_dotted_i", { enabled });
    onRefresh();
  };

  const updateTriggers = async (patch: Partial<AppSettingsDto["trigger_config"]>) => {
    if (!settings) return;
    const next = { ...settings.trigger_config, ...patch };
    setSettings({ ...settings, trigger_config: next });
    await invoke("set_trigger_config", { config: next });
    setBusyMsg(t("settings.saved"));
  };

  const updateMascotPrefs = async (
    patch: Partial<{
      sound: boolean;
      size: AppSettingsDto["mascot_size"];
      sound_baa: boolean;
      sound_throw: boolean;
      sound_impact: boolean;
    }>,
  ) => {
    if (!settings) return;
    const sound = patch.sound ?? settings.mascot_sound;
    const size = patch.size ?? settings.mascot_size;
    const soundBaa = patch.sound_baa ?? settings.sound_baa;
    const soundThrow = patch.sound_throw ?? settings.sound_throw;
    const soundImpact = patch.sound_impact ?? settings.sound_impact;
    setSettings({
      ...settings,
      mascot_sound: sound,
      mascot_size: size,
      sound_baa: soundBaa,
      sound_throw: soundThrow,
      sound_impact: soundImpact,
    });
    await invoke("set_mascot_prefs", {
      sound,
      size,
      soundBaa,
      soundThrow,
      soundImpact,
    });
    setBusyMsg(t("settings.saved"));
  };

  const saveShortcuts = async (next: ShortcutConfigDto) => {
    setErrMsg(null);
    setBusyMsg(null);
    try {
      await invoke("set_shortcuts_config", { shortcuts: next });
      setShortcutDraft(next);
      setSettings((prev) => (prev ? { ...prev, shortcuts: next } : prev));
      setBusyMsg(t("settings.saved"));
    } catch (e) {
      setErrMsg(String(e));
    }
  };

  const saveExcluded = async (nextList: string[]) => {
    setErrMsg(null);
    const updated = await invoke<string[]>("set_excluded_processes", {
      processes: nextList,
    });
    setSettings((prev) =>
      prev ? { ...prev, excluded_processes: updated } : prev,
    );
    setBusyMsg(t("settings.saved"));
  };

  const handleBackupNow = async () => {
    setErrMsg(null);
    setBusyMsg(null);
    try {
      const info = await invoke<BackupFileInfo>("create_backup_now");
      setBusyMsg(t("settings.backupCreated", { name: info.filename }));
      await loadSettings();
    } catch (e) {
      setErrMsg(String(e));
    }
  };

  const handleRestore = async (filename: string) => {
    setErrMsg(null);
    setBusyMsg(null);
    try {
      const safety = await invoke<BackupFileInfo>("restore_backup", {
        filename,
      });
      setConfirmRestore(null);
      setBusyMsg(
        t("settings.restoreDone", { name: filename, safety: safety.filename }),
      );
      await loadSettings();
      onRefresh();
    } catch (e) {
      setErrMsg(String(e));
    }
  };

  return (
    <div className="settings-shell">
      <aside className="settings-sidebar">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`settings-nav-item ${section === item.id ? "is-active" : ""}`}
            onClick={() => {
              setBusyMsg(null);
              setErrMsg(null);
              setConfirmRestore(null);
              setSection(item.id);
              if (item.id === "excluded") {
                void invoke<RunningAppInfo[]>("list_running_apps")
                  .then(setRunningApps)
                  .catch(() => {});
              }
            }}
          >
            {t(item.labelKey)}
          </button>
        ))}
      </aside>

      <main className="settings-content">
        {section !== "appearance" && errMsg && (
          <div className="error-banner" style={{ marginBottom: 12 }}>
            {errMsg}
          </div>
        )}
        {section !== "appearance" && busyMsg && (
          <div className="success-banner" style={{ marginBottom: 12 }}>
            {busyMsg}
          </div>
        )}

        {section === "appearance" ? (
          <AppearanceSettings />
        ) : section === "general" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.general")}</h2>
            <div className="settings-card">
              <div className="settings-row">
                <div>
                  <div className="card-title">{t("settings.engineStateTitle")}</div>
                  <div className="muted small">
                    {status?.paused ? t("status.paused") : t("status.active")} ·{" "}
                    {t("status.phraseCount", {
                      count: status?.active_phrase_count ?? 0,
                    })}
                  </div>
                </div>
                <button
                  type="button"
                  className="btn"
                  onClick={() => void togglePause()}
                >
                  {status?.paused ? t("status.resume") : t("status.pause")}
                </button>
              </div>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.userNameTitle")}</div>
                  <div className="muted small">{t("settings.userNameDesc")}</div>
                </div>
                <div className="row-actions">
                  <input
                    className="input"
                    style={{ width: 180 }}
                    value={userNameDraft}
                    onChange={(e) => setUserNameDraft(e.target.value)}
                    placeholder={t("onboarding.namePlaceholder")}
                  />
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={async () => {
                      await invoke("set_user_profile", {
                        userName: userNameDraft,
                      });
                      setBusyMsg(t("settings.saved"));
                    }}
                  >
                    {t("form.save")}
                  </button>
                </div>
              </div>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.dataDirTitle")}</div>
                  <div className="muted small">{status?.data_dir || "—"}</div>
                </div>
                <div className="row-actions">
                  <span className="badge">v{status?.version ?? "1.1.0"}</span>
                  <button
                    type="button"
                    className="btn btn-sm"
                    title={t("settings.dataDirTitle")}
                    onClick={() => void invoke("open_data_dir")}
                  >
                    {t("settings.openDataDirBtn")}
                  </button>
                  <button
                    type="button"
                    className="icon-btn"
                    title={t("settings.openLogDir")}
                    aria-label={t("settings.openLogDir")}
                    onClick={() => void invoke("open_log_dir")}
                  >
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                      <path
                        d="M4 1.75h5.5L12.25 4.5v9.75H4z M9.25 1.75V4.75h3 M6 7.5h4.5 M6 9.75h4.5 M6 12h2.5"
                        stroke="currentColor"
                        strokeWidth="1.25"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  </button>
                </div>
              </div>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.clipboardRememberTitle")}</div>
                  <div className="muted small">{t("settings.clipboardRememberDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={settings?.clipboard_enabled ?? true}
                  onChange={async (e) => {
                    if (!settings) return;
                    const enabled = e.target.checked;
                    const limit = settings.clipboard_limit || 10;
                    setSettings({ ...settings, clipboard_enabled: enabled });
                    await invoke("set_clipboard_prefs", { enabled, limit });
                    setBusyMsg(t("settings.saved"));
                  }}
                />
              </label>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.clipboardLimitTitle")}</div>
                  <div className="muted small">{t("settings.clipboardLimitDesc")}</div>
                </div>
                <select
                  className="select"
                  style={{ width: 120 }}
                  value={settings?.clipboard_limit ?? 10}
                  disabled={!(settings?.clipboard_enabled ?? true)}
                  onChange={async (e) => {
                    if (!settings) return;
                    const limit = Number(e.target.value);
                    const enabled = settings.clipboard_enabled;
                    setSettings({ ...settings, clipboard_limit: limit });
                    await invoke("set_clipboard_prefs", { enabled, limit });
                    onRefresh();
                    setBusyMsg(t("settings.saved"));
                  }}
                >
                  {[10, 25, 50, 100].map((n) => (
                    <option key={n} value={n}>
                      {n}
                    </option>
                  ))}
                </select>
              </div>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.abbrSuggestTitle")}</div>
                  <div className="muted small">{t("settings.abbrSuggestDesc")}</div>
                </div>
                <select
                  className="select"
                  style={{ width: 150 }}
                  value={settings?.abbr_suggest_mode ?? "initials"}
                  onChange={async (e) => {
                    if (!settings) return;
                    const mode = e.target.value as AbbrSuggestMode;
                    setSettings({ ...settings, abbr_suggest_mode: mode });
                    await invoke("set_abbr_suggest_mode", { mode });
                    setBusyMsg(t("settings.saved"));
                  }}
                >
                  <option value="initials">{t("settings.abbrMode.initials")}</option>
                  <option value="first_word">{t("settings.abbrMode.first_word")}</option>
                  <option value="last_word">{t("settings.abbrMode.last_word")}</option>
                  <option value="tag">{t("settings.abbrMode.tag")}</option>
                </select>
              </div>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.runAtLoginTitle")}</div>
                  <div className="muted small">{t("settings.runAtLoginDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={Boolean(startup?.run_at_login)}
                  onChange={async (e) => {
                    setErrMsg(null);
                    try {
                      const next = await invoke<StartupStatus>("set_run_at_login", {
                        enabled: e.target.checked,
                      });
                      setStartup(next);
                      setBusyMsg(t("settings.saved"));
                    } catch (err) {
                      setErrMsg(String(err));
                    }
                  }}
                />
              </label>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">
                    {t("settings.adminTaskTitle")}
                    {startup?.is_elevated && (
                      <span className="badge" style={{ marginLeft: 8 }}>
                        {t("settings.elevatedNow")}
                      </span>
                    )}
                  </div>
                  <div className="muted small">{t("settings.adminTaskDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={Boolean(startup?.admin_task_enabled)}
                  onChange={async (e) => {
                    setErrMsg(null);
                    try {
                      const next = await invoke<StartupStatus>(
                        "set_admin_autostart",
                        { enabled: e.target.checked },
                      );
                      setStartup(next);
                      setBusyMsg(t("settings.saved"));
                    } catch (err) {
                      setErrMsg(String(err));
                    }
                  }}
                />
              </label>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.replayTourTitle")}</div>
                  <div className="muted small">{t("settings.replayTourDesc")}</div>
                </div>
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={onReplayOnboarding}
                >
                  {t("settings.replayTourBtn")}
                </button>
              </div>
            </div>
          </div>
        ) : section === "trigger" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.trigger")}</h2>
            <p className="muted small">{t("settings.triggerDesc")}</p>
            <div className="settings-card">
              {(
                [
                  ["space", "settings.trigger.space"],
                  ["enter", "settings.trigger.enter"],
                  ["tab", "settings.trigger.tab"],
                  ["punctuation", "settings.trigger.punctuation"],
                ] as const
              ).map(([key, labelKey]) => (
                <label
                  key={key}
                  className="settings-row trigger-check-row"
                  style={{ cursor: "pointer" }}
                >
                  <span>{t(labelKey)}</span>
                  <input
                    type="checkbox"
                    checked={settings?.trigger_config[key] ?? true}
                    onChange={(e) =>
                      void updateTriggers({ [key]: e.target.checked })
                    }
                  />
                </label>
              ))}
              <label
                className="settings-row trigger-check-row"
                style={{ cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.suggestEnabledTitle")}</div>
                  <div className="muted small">{t("settings.suggestEnabledDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={settings?.suggest_enabled ?? true}
                  onChange={async (e) => {
                    if (!settings) return;
                    const enabled = e.target.checked;
                    setSettings({ ...settings, suggest_enabled: enabled });
                    await invoke("set_suggest_enabled", { enabled });
                    setBusyMsg(t("settings.saved"));
                  }}
                />
              </label>
            </div>
          </div>
        ) : section === "case" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.case")}</h2>
            <div className="settings-card">
              <label className="settings-row" style={{ cursor: "pointer" }}>
                <div>
                  <div className="card-title">{t("status.foldDottedI")}</div>
                  <div className="muted small">{t("settings.foldIDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={Boolean(status?.fold_dotted_i)}
                  onChange={(e) => void toggleFoldI(e.target.checked)}
                />
              </label>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.shiftF3Title")}</div>
                  <div className="muted small">{t("settings.shiftF3Desc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={settings?.shift_f3_enabled ?? true}
                  onChange={async (e) => {
                    if (!settings) return;
                    const enabled = e.target.checked;
                    setSettings({ ...settings, shift_f3_enabled: enabled });
                    await invoke("set_shift_f3_enabled", { enabled });
                    setBusyMsg(t("settings.saved"));
                  }}
                />
              </label>

              <div className="case-examples-grid">
                <div className="case-example-row">
                  <span className="abbr-chip">syg</span>
                  <span>→ saygılarımla</span>
                </div>
                <div className="case-example-row">
                  <span className="abbr-chip">Syg</span>
                  <span>→ Saygılarımla</span>
                </div>
                <div className="case-example-row">
                  <span className="abbr-chip">SYG</span>
                  <span>→ SAYGILARIMLA</span>
                </div>
              </div>
            </div>
          </div>
        ) : section === "mascot" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.mascot")}</h2>
            <div className="settings-card">
              <div className="settings-row">
                <div>
                  <div className="card-title">{t("settings.mascotVisibility")}</div>
                  <div className="muted small">{t("settings.mascotVisibilityDesc")}</div>
                </div>
                <button
                  type="button"
                  className="btn"
                  onClick={() => void invoke("toggle_mascot_visible")}
                >
                  {t("settings.mascotToggleBtn")}
                </button>
              </div>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("settings.mascotSizeTitle")}</div>
                  <div className="muted small">{t("settings.mascotSizeDesc")}</div>
                </div>
                <div className="segmented-control">
                  {(["small", "normal", "large"] as const).map((sz) => (
                    <button
                      key={sz}
                      type="button"
                      className={`seg-btn ${settings?.mascot_size === sz ? "is-active" : ""}`}
                      onClick={() => void updateMascotPrefs({ size: sz })}
                    >
                      {t(`settings.size.${sz}` as TranslationKey)}
                    </button>
                  ))}
                </div>
              </div>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.mascotMasterSoundTitle")}</div>
                  <div className="muted small">{t("settings.mascotMasterSoundDesc")}</div>
                </div>
                <input
                  type="checkbox"
                  checked={settings?.mascot_sound ?? true}
                  onChange={(e) =>
                    void updateMascotPrefs({ sound: e.target.checked })
                  }
                />
              </label>

              <label
                className="settings-row"
                style={{ marginTop: 12, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.mascotSoundTitle")}</div>
                  <div className="muted small">{t("settings.mascotSoundDesc")}</div>
                </div>
                <div className="row-actions">
                  <button
                    type="button"
                    className="btn btn-sm"
                    title={t("settings.testBaaBtn")}
                    onClick={(e) => {
                      e.preventDefault();
                      playBaaSound();
                    }}
                  >
                    {t("settings.testBaaBtn")}
                  </button>
                  <input
                    type="checkbox"
                    disabled={!(settings?.mascot_sound ?? true)}
                    checked={settings?.sound_baa ?? false}
                    onChange={(e) =>
                      void updateMascotPrefs({ sound_baa: e.target.checked })
                    }
                  />
                </div>
              </label>

              <label
                className="settings-row"
                style={{ marginTop: 12, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.soundThrowTitle")}</div>
                  <div className="muted small">{t("settings.soundThrowDesc")}</div>
                </div>
                <div className="row-actions">
                  <button
                    type="button"
                    className="btn btn-sm"
                    title={t("settings.testThrowBtn")}
                    onClick={(e) => {
                      e.preventDefault();
                      playThrowSound();
                    }}
                  >
                    {t("settings.testThrowBtn")}
                  </button>
                  <input
                    type="checkbox"
                    disabled={!(settings?.mascot_sound ?? true)}
                    checked={settings?.sound_throw ?? true}
                    onChange={(e) =>
                      void updateMascotPrefs({ sound_throw: e.target.checked })
                    }
                  />
                </div>
              </label>

              <label
                className="settings-row"
                style={{ marginTop: 12, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">{t("settings.soundImpactTitle")}</div>
                  <div className="muted small">{t("settings.soundImpactDesc")}</div>
                </div>
                <div className="row-actions">
                  <button
                    type="button"
                    className="btn btn-sm"
                    title={t("settings.testImpactBtn")}
                    onClick={(e) => {
                      e.preventDefault();
                      playWallImpactSound(1100);
                    }}
                  >
                    {t("settings.testImpactBtn")}
                  </button>
                  <input
                    type="checkbox"
                    disabled={!(settings?.mascot_sound ?? true)}
                    checked={settings?.sound_impact ?? true}
                    onChange={(e) =>
                      void updateMascotPrefs({ sound_impact: e.target.checked })
                    }
                  />
                </div>
              </label>

              <div className="settings-row" style={{ marginTop: 14 }}>
                <div>
                  <div className="card-title">{t("mascot.menuBasketStart")}</div>
                  <div className="muted small">{t("settings.basketDesc")}</div>
                </div>
                <button
                  type="button"
                  className="btn"
                  onClick={() => void invoke("toggle_basketball_game")}
                >
                  {t("settings.basketToggleBtn")}
                </button>
              </div>
            </div>
          </div>
        ) : section === "shortcuts" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.shortcuts")}</h2>
            {shortcutConflict && (
              <div className="error-banner">{t("settings.shortcutConflict")}</div>
            )}
            <div className="settings-card">
              <div className="shortcut-list">
                {(
                  [
                    ["pause", "settings.shortcut.pause"],
                    ["search", "settings.shortcut.search"],
                    ["quick_add", "settings.shortcut.quickAdd"],
                    ["studio", "settings.shortcut.studio"],
                    ["toggle_mascot", "settings.shortcut.toggleMascot"],
                  ] as const
                ).map(([field, labelKey]) => (
                  <div key={field} className="settings-row">
                    <span>{t(labelKey)}</span>
                    <input
                      className="input"
                      readOnly
                      style={{
                        width: 240,
                        fontFamily: "ui-monospace, Consolas, monospace",
                        cursor: "pointer",
                      }}
                      title={t("settings.shortcutFieldTip")}
                      placeholder={t("settings.shortcutCapturePlaceholder")}
                      value={shortcutDraft[field]}
                      onKeyDown={(e) => {
                        e.preventDefault();
                        const combo = captureShortcutCombo(e);
                        if (combo) {
                          setShortcutDraft({
                            ...shortcutDraft,
                            [field]: combo,
                          });
                        }
                      }}
                    />
                  </div>
                ))}
              </div>
              <div className="row-actions" style={{ marginTop: 14, justifyContent: "flex-end" }}>
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => void saveShortcuts(DEFAULT_SHORTCUTS)}
                >
                  {t("settings.resetDefaults")}
                </button>
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  disabled={shortcutConflict}
                  onClick={() => void saveShortcuts(shortcutDraft)}
                >
                  {t("form.save")}
                </button>
              </div>
            </div>
          </div>
        ) : section === "excluded" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.excluded")}</h2>
            <p className="muted small">{t("settings.info.excluded")}</p>
            <div className="settings-card">
              <div className="row-actions">
                <input
                  className="input"
                  value={newProc}
                  onChange={(e) => setNewProc(e.target.value)}
                  placeholder="örnek.exe"
                />
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  onClick={() => {
                    const p = newProc.trim();
                    if (!p) return;
                    const current = settings?.excluded_processes ?? [];
                    if (!current.includes(p.toLowerCase())) {
                      void saveExcluded([...current, p]);
                    }
                    setNewProc("");
                  }}
                >
                  {t("settings.addProcess")}
                </button>
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => void saveExcluded(DEFAULT_EXCLUDED)}
                >
                  {t("settings.resetDefaults")}
                </button>
              </div>

              <div className="excluded-chips-wrap">
                {(settings?.excluded_processes ?? []).map((proc) => (
                  <span key={proc} className="tag-pill">
                    {proc}
                    <button
                      type="button"
                      onClick={() =>
                        void saveExcluded(
                          (settings?.excluded_processes ?? []).filter(
                            (p) => p !== proc,
                          ),
                        )
                      }
                    >
                      ×
                    </button>
                  </span>
                ))}
              </div>

              {runningApps.length > 0 && (
                <div style={{ marginTop: 12 }}>
                  <div className="row-actions" style={{ justifyContent: "space-between", marginBottom: 6 }}>
                    <span className="muted small">{t("studio.detectApps")}</span>
                    <button
                      type="button"
                      className="link-btn"
                      onClick={() =>
                        void invoke<RunningAppInfo[]>("list_running_apps")
                          .then(setRunningApps)
                          .catch(() => {})
                      }
                    >
                      {t("studio.refreshApps")}
                    </button>
                  </div>
                  <div className="excluded-chips-wrap">
                    {runningApps.map((app) => {
                      const low = app.process_name.toLowerCase();
                      const already = (settings?.excluded_processes ?? []).includes(low);
                      return (
                        <button
                          key={app.process_name}
                          type="button"
                          className={`btn btn-sm ${already ? "is-active-outline" : ""}`}
                          title={app.window_title}
                          onClick={() => {
                            const cur = settings?.excluded_processes ?? [];
                            if (already) {
                              void saveExcluded(cur.filter((x) => x !== low));
                            } else {
                              void saveExcluded([...cur, low]);
                            }
                          }}
                        >
                          {already ? "✓ " : "+ "}
                          {app.process_name}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>
        ) : section === "backup" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.backup")}</h2>
            <div className="settings-card">
              <div className="settings-row">
                <div>
                  <div className="card-title">{t("settings.backupNowTitle")}</div>
                  <div className="muted small">{t("settings.backupNowDesc")}</div>
                </div>
                <div className="row-actions">
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    onClick={() => void handleBackupNow()}
                  >
                    {t("settings.backupNowBtn")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={() => {
                      void onExport("meetin").then(() =>
                        setBusyMsg(t("settings.backupDone")),
                      );
                    }}
                  >
                    {t("studio.exportMeetin")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm"
                    onClick={() => void onExport("csv")}
                  >
                    {t("studio.exportCsv")}
                  </button>
                </div>
              </div>
            </div>

            <div className="settings-card">
              <div className="card-title">{t("settings.backupListTitle")}</div>
              <p className="muted small" style={{ marginTop: 2 }}>
                {t("settings.backupSafeRestoreHint")}
              </p>
              {backups.length === 0 ? (
                <p className="muted small">{t("settings.backupEmpty")}</p>
              ) : (
                <div className="backup-list">
                  {backups.map((b) => (
                    <div key={b.filename} className="backup-row">
                      <div>
                        <div className="card-title">{b.filename}</div>
                        <div className="muted small">
                          {b.modified_at} · {Math.max(1, Math.round(b.size_bytes / 1024))} KB
                        </div>
                      </div>
                      {confirmRestore === b.filename ? (
                        <div className="row-actions">
                          <button
                            type="button"
                            className="btn btn-danger btn-sm"
                            onClick={() => void handleRestore(b.filename)}
                          >
                            {t("settings.confirmRestoreBtn")}
                          </button>
                          <button
                            type="button"
                            className="btn btn-sm"
                            onClick={() => setConfirmRestore(null)}
                          >
                            {t("form.cancel")}
                          </button>
                        </div>
                      ) : (
                        <button
                          type="button"
                          className="btn btn-sm"
                          onClick={() => setConfirmRestore(b.filename)}
                        >
                          {t("settings.restoreBtn")}
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ) : section === "update" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.update")}</h2>
            <div className="settings-card">
              <div className="settings-row">
                <div>
                  <div className="card-title">
                    {t("settings.currentVersionTitle")} {updateInfo ? `v${updateInfo.current_version}` : ""}
                  </div>
                  <div className="muted small">
                    {updateInfo?.is_portable
                      ? t("settings.modePortable")
                      : t("settings.modeInstalled")}{" "}
                    · WebView2:{" "}
                    {updateInfo?.webview2_installed
                      ? t("settings.webviewOk")
                      : t("settings.webviewMissing")}
                  </div>
                </div>
                {updateInfo?.latest_version && (
                  <span className="badge">
                    {updateInfo.update_available
                      ? `v${updateInfo.latest_version}`
                      : t("settings.upToDate")}
                  </span>
                )}
              </div>

              <label
                className="settings-row"
                style={{ marginTop: 14, cursor: "pointer" }}
              >
                <div>
                  <div className="card-title">
                    {t("settings.autoCheckUpdateTitle")}
                  </div>
                  <div className="muted small">
                    {t("settings.autoCheckUpdateDesc")}
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={settings?.auto_check_updates ?? true}
                  onChange={async (e) => {
                    if (!settings) return;
                    const enabled = e.target.checked;
                    setSettings({ ...settings, auto_check_updates: enabled });
                    await invoke("set_auto_check_updates", { enabled });
                    setBusyMsg(t("settings.saved"));
                  }}
                />
              </label>

              <div className="row-actions" style={{ marginTop: 14 }}>
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  disabled={checkingUpdate || installingUpdate}
                  onClick={async () => {
                    setErrMsg(null);
                    setBusyMsg(null);
                    setCheckingUpdate(true);
                    try {
                      const res =
                        await invoke<UpdateCheckResult>("check_for_updates");
                      setUpdateInfo(res);
                      setBusyMsg(
                        res.update_available
                          ? t("settings.updateFound", {
                              ver: res.latest_version ?? "",
                            })
                          : t("settings.upToDate"),
                      );
                    } catch (err) {
                      setErrMsg(String(err));
                    } finally {
                      setCheckingUpdate(false);
                    }
                  }}
                >
                  {checkingUpdate
                    ? t("settings.checkingUpdate")
                    : t("settings.checkUpdateBtn")}
                </button>
              </div>

              {updateInfo?.update_available && (
                <div className="case-examples-grid">
                  <div className="update-header-row">
                    <strong>
                      {t("settings.updateFound", {
                        ver: updateInfo.latest_version ?? "",
                      })}
                    </strong>
                  </div>

                  {updateInfo.notes && (
                    <pre
                      className="muted small"
                      style={{ whiteSpace: "pre-wrap", margin: 0 }}
                    >
                      {updateInfo.notes}
                    </pre>
                  )}

                  {!updateInfo.can_auto_install && (
                    <div className="muted small">
                      {t("settings.updateNoAutoInstallHint")}
                    </div>
                  )}

                  <div className="row-actions" style={{ marginTop: 8 }}>
                    {updateInfo.can_auto_install && (
                    <button
                      type="button"
                      className="btn btn-primary btn-sm"
                      disabled={installingUpdate}
                      onClick={async () => {
                        setErrMsg(null);
                        setBusyMsg(t("settings.updatingProgress"));
                        setInstallingUpdate(true);
                        try {
                          const res = await invoke<UpdateInstallResult>(
                            "install_verified_update",
                          );
                          setBusyMsg(
                            t("settings.updateRestarting", {
                              ver: res.version,
                            }),
                          );
                        } catch (err) {
                          setBusyMsg(null);
                          setErrMsg(String(err));
                          setInstallingUpdate(false);
                        }
                      }}
                    >
                      {installingUpdate
                        ? t("settings.updatingProgress")
                        : t("settings.downloadAndUpdateBtn")}
                    </button>
                    )}

                    {!updateInfo.can_auto_install &&
                      updateInfo.download_url && (
                        <a
                          className="btn btn-sm"
                          href={updateInfo.download_url}
                          target="_blank"
                          rel="noreferrer"
                        >
                          {t("settings.manualDownloadBtn")}
                        </a>
                      )}

                    <button
                      type="button"
                      className="btn btn-sm"
                      disabled={installingUpdate}
                      onClick={() =>
                        setUpdateInfo({
                          ...updateInfo,
                          update_available: false,
                        })
                      }
                    >
                      {t("settings.laterBtn")}
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        ) : section === "language" ? (
          <div className="settings-page">
            <h2>{t("settings.nav.language")}</h2>
            <div className="settings-card">
              <div className="settings-row">
                <div>
                  <div className="card-title">{t("settings.languageTitle")}</div>
                  <div className="muted small">{t("settings.languageDesc")}</div>
                </div>
                <div className="segmented-control" role="radiogroup" aria-label={t("settings.languageTitle")}>
                  {(["tr", "en"] as const).map((langOpt: Language) => (
                    <button
                      key={langOpt}
                      type="button"
                      role="radio"
                      aria-checked={activeLanguage === langOpt}
                      className={`seg-btn ${activeLanguage === langOpt ? "is-active" : ""}`}
                      onClick={async () => {
                        setErrMsg(null);
                        setLanguage(langOpt);
                        setSettings((prev) =>
                          prev ? { ...prev, language: langOpt } : prev,
                        );
                        try {
                          await invoke<string>("set_language", {
                            language: langOpt,
                          });
                          setBusyMsg(t("settings.saved"));
                        } catch (err) {
                          setErrMsg(String(err));
                        }
                      }}
                    >
                      {t(`settings.lang.${langOpt}` as TranslationKey)}
                    </button>
                  ))}
                </div>
              </div>

              <div className="source-cards" style={{ marginTop: 14 }}>
                {(["tr", "en"] as const).map((langOpt: Language) => {
                  const isActive = activeLanguage === langOpt;
                  return (
                    <button
                      key={langOpt}
                      type="button"
                      className={`source-card ${isActive ? "is-active" : ""}`}
                      onClick={async () => {
                        setErrMsg(null);
                        setLanguage(langOpt);
                        setSettings((prev) =>
                          prev ? { ...prev, language: langOpt } : prev,
                        );
                        try {
                          await invoke<string>("set_language", {
                            language: langOpt,
                          });
                          setBusyMsg(t("settings.saved"));
                        } catch (err) {
                          setErrMsg(String(err));
                        }
                      }}
                    >
                      <div className="source-radio">
                        <span className={`radio-dot ${isActive ? "is-checked" : ""}`} />
                        <strong>{t(`settings.lang.${langOpt}` as TranslationKey)}</strong>
                      </div>
                      <span className="muted small">
                        {t(`settings.lang.${langOpt}Desc` as TranslationKey)}
                      </span>
                    </button>
                  );
                })}
              </div>

              <p className="muted small" style={{ margin: "12px 0 0" }}>
                {t("settings.languageLiveHint")}
              </p>
            </div>
          </div>
        ) : (
          <div className="settings-page">
            <h2>{t(`settings.nav.${section}`)}</h2>
            <div className="settings-card">
              <p className="muted">
                {t(`settings.info.${section}` as TranslationKey)}
              </p>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
