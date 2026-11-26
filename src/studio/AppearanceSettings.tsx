import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  applyThemeToDocument,
  contrastRatio,
  DEFAULT_THEME_CONFIG,
  hexWithHue,
  hueFromHex,
  normalizeHex,
  PRESET_ACCENTS,
  resolveThemeTokens,
  SCALE_STEPS,
  THEME_ROLES,
  type FontSizeOption,
  type IslandSurface,
  type PaletteSource,
  type ThemeConfig,
  type ThemeMode,
  type ThemeRoleKey,
} from "../design/palette";
import { t, type TranslationKey } from "../i18n";
import { MeetinSheep } from "../mascot/MeetinSheep";

interface SystemThemeInfo {
  dark: boolean;
  accent_hex: string;
}

const ROLE_LABEL_KEYS: Record<
  ThemeRoleKey,
  { name: TranslationKey; desc: TranslationKey }
> = {
  "--vurgu": {
    name: "settings.role.vurgu",
    desc: "settings.role.vurguDesc",
  },
  "--vurgu-ustu": {
    name: "settings.role.vurguUstu",
    desc: "settings.role.vurguUstuDesc",
  },
  "--vurgu-zayif": {
    name: "settings.role.vurguZayif",
    desc: "settings.role.vurguZayifDesc",
  },
  "--zemin": {
    name: "settings.role.zemin",
    desc: "settings.role.zeminDesc",
  },
  "--yuzey": {
    name: "settings.role.yuzey",
    desc: "settings.role.yuzeyDesc",
  },
  "--yuzey-2": {
    name: "settings.role.yuzey2",
    desc: "settings.role.yuzey2Desc",
  },
  "--cizgi": {
    name: "settings.role.cizgi",
    desc: "settings.role.cizgiDesc",
  },
  "--metin": {
    name: "settings.role.metin",
    desc: "settings.role.metinDesc",
  },
  "--metin-ikincil": {
    name: "settings.role.metinIkincil",
    desc: "settings.role.metinIkincilDesc",
  },
  "--metin-pasif": {
    name: "settings.role.metinPasif",
    desc: "settings.role.metinPasifDesc",
  },
  "--vurgu-hover": {
    name: "settings.role.vurguHover",
    desc: "settings.role.vurguHoverDesc",
  },
  "--vurgu-basili": {
    name: "settings.role.vurguBasili",
    desc: "settings.role.vurguBasiliDesc",
  },
  "--basari": {
    name: "settings.role.basari",
    desc: "settings.role.basariDesc",
  },
  "--uyari": {
    name: "settings.role.uyari",
    desc: "settings.role.uyariDesc",
  },
  "--hata": {
    name: "settings.role.hata",
    desc: "settings.role.hataDesc",
  },
};

export function AppearanceSettings() {
  const [config, setConfig] = useState<ThemeConfig>(DEFAULT_THEME_CONFIG);
  const [sysTheme, setSysTheme] = useState<SystemThemeInfo>({
    dark: false,
    accent_hex: "#0078D4",
  });
  const [hexDraft, setHexDraft] = useState(DEFAULT_THEME_CONFIG.accentHex);
  const [notice, setNotice] = useState<string | null>(null);
  const importRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    Promise.all([
      invoke<SystemThemeInfo>("get_system_theme").catch(() => sysTheme),
      invoke<string | null>("get_theme_config").catch(() => null),
    ]).then(([sys, cfgRaw]) => {
      setSysTheme(sys);
      if (cfgRaw) {
        try {
          const parsed = JSON.parse(cfgRaw) as Partial<ThemeConfig>;
          const next: ThemeConfig = {
            ...DEFAULT_THEME_CONFIG,
            ...parsed,
            overrides: { ...(parsed.overrides ?? {}) },
          };
          setConfig(next);
          setHexDraft(next.accentHex);
        } catch {
          // Varsayılan tema korunur
        }
      }
    });
  }, []);

  const resolvedTheme = useMemo(
    () => resolveThemeTokens(config, sysTheme.dark, sysTheme.accent_hex),
    [config, sysTheme],
  );

  const commitConfig = (next: ThemeConfig) => {
    setConfig(next);
    const res = resolveThemeTokens(next, sysTheme.dark, sysTheme.accent_hex);
    applyThemeToDocument(res, next.fontSize ?? "md");
    void invoke("save_theme_config", {
      configJson: JSON.stringify(next),
    });
  };

  const setFontSize = (fontSize: FontSizeOption) =>
    commitConfig({ ...config, fontSize });
  const setMode = (mode: ThemeMode) => commitConfig({ ...config, mode });
  const setSource = (source: PaletteSource) =>
    commitConfig({ ...config, source });
  const setAccent = (hex: string) => {
    const norm = normalizeHex(hex, config.accentHex);
    setHexDraft(norm);
    commitConfig({ ...config, source: "custom", accentHex: norm });
  };

  const setRoleOverride = (role: ThemeRoleKey, hex: string) => {
    const norm = normalizeHex(hex, resolvedTheme.resolved[role]);
    commitConfig({
      ...config,
      overrides: { ...config.overrides, [role]: norm },
    });
  };

  const clearRoleOverride = (role: ThemeRoleKey) => {
    const nextOverrides = { ...config.overrides };
    delete nextOverrides[role];
    commitConfig({ ...config, overrides: nextOverrides });
  };

  const resetAllOverrides = () => {
    commitConfig({ ...config, overrides: {} });
  };

  const resetDefaults = () => {
    setHexDraft(DEFAULT_THEME_CONFIG.accentHex);
    commitConfig(DEFAULT_THEME_CONFIG);
  };

  const handleExportTheme = () => {
    const blob = new Blob([JSON.stringify(config, null, 2)], {
      type: "application/json;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "meetin-tema.mee-tema";
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  const handleImportTheme = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    try {
      const text = await file.text();
      const parsed = JSON.parse(text) as Partial<ThemeConfig>;
      const next: ThemeConfig = {
        ...DEFAULT_THEME_CONFIG,
        ...parsed,
        overrides: { ...(parsed.overrides ?? {}) },
      };
      setHexDraft(next.accentHex);
      commitConfig(next);
      setNotice(t("settings.themeImported"));
    } catch {
      setNotice(t("settings.themeImportError"));
    }
  };

  const currentHue = hueFromHex(config.accentHex);

  return (
    <div className="appearance-layout">
      <div className="appearance-controls">
        <div className="settings-head">
          <h2>{t("settings.appearanceTitle")}</h2>
          <p className="muted small">{t("settings.appearanceSub")}</p>
        </div>

        {notice && <div className="success-banner">{notice}</div>}

        <section className="settings-card">
          <div className="settings-row">
            <div>
              <div className="card-title">{t("settings.appModeTitle")}</div>
              <div className="muted small">{t("settings.appModeDesc")}</div>
            </div>
            <div className="segmented-control">
              {(["system", "light", "dark"] as ThemeMode[]).map((m) => (
                <button
                  key={m}
                  type="button"
                  className={`seg-btn ${config.mode === m ? "is-active" : ""}`}
                  onClick={() => setMode(m)}
                >
                  {t(`settings.mode.${m}` as TranslationKey)}
                </button>
              ))}
            </div>
          </div>

          <div className="settings-row" style={{ marginTop: 12 }}>
            <div>
              <div className="card-title">{t("settings.fontSizeTitle")}</div>
              <div className="muted small">{t("settings.fontSizeDesc")}</div>
            </div>
            <div className="segmented-control">
              {(["sm", "md", "lg", "xl"] as FontSizeOption[]).map((sz) => (
                <button
                  key={sz}
                  type="button"
                  className={`seg-btn ${(config.fontSize ?? "md") === sz ? "is-active" : ""}`}
                  onClick={() => setFontSize(sz)}
                >
                  {t(`settings.fontSize.${sz}` as TranslationKey)}
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className="settings-card">
          <div className="card-title">{t("settings.paletteTitle")}</div>
          <div className="source-cards">
            <button
              type="button"
              className={`source-card ${config.source === "system" ? "is-active" : ""}`}
              onClick={() => setSource("system")}
            >
              <div className="source-radio">
                <span
                  className={`radio-dot ${config.source === "system" ? "is-checked" : ""}`}
                />
                <strong>{t("settings.sourceSystem")}</strong>
              </div>
              <span className="muted small">
                {t("settings.sourceSystemDesc")} ({sysTheme.accent_hex})
              </span>
            </button>

            <button
              type="button"
              className={`source-card ${config.source === "custom" ? "is-active" : ""}`}
              onClick={() => setSource("custom")}
            >
              <div className="source-radio">
                <span
                  className={`radio-dot ${config.source === "custom" ? "is-checked" : ""}`}
                />
                <strong>{t("settings.sourceCustom")}</strong>
              </div>
              <span className="muted small">
                {t("settings.sourceCustomDesc")}
              </span>
            </button>
          </div>

          <div className="accent-picker-row">
            <div className="preset-circles">
              {PRESET_ACCENTS.map((preset) => {
                const isSelected =
                  config.source === "custom" &&
                  config.accentHex.toUpperCase() === preset.hex.toUpperCase();
                return (
                  <button
                    key={preset.hex}
                    type="button"
                    className={`preset-circle ${isSelected ? "is-active" : ""}`}
                    style={{ backgroundColor: preset.hex }}
                    title={preset.name}
                    onClick={() => setAccent(preset.hex)}
                  />
                );
              })}
            </div>

            <div className="hex-box">
              <input
                type="color"
                className="color-input-swatch"
                value={normalizeHex(config.accentHex)}
                onChange={(e) => setAccent(e.target.value)}
              />
              <input
                className="hex-text-input"
                value={hexDraft}
                onChange={(e) => {
                  const val = e.target.value;
                  setHexDraft(val);
                  if (/^#?[0-9a-fA-F]{6}$/.test(val.trim())) {
                    setAccent(val);
                  }
                }}
                onBlur={() => setHexDraft(config.accentHex)}
              />
            </div>
          </div>

          <label className="hue-slider-row">
            <span className="muted small">{t("settings.hueLabel")}</span>
            <input
              type="range"
              min={0}
              max={359}
              value={currentHue}
              className="hue-slider"
              onChange={(e) =>
                setAccent(hexWithHue(config.accentHex, Number(e.target.value)))
              }
            />
          </label>
        </section>

        <section className="settings-card">
          <div className="card-title">{t("settings.scaleTitle")}</div>
          <div className="scale-strip">
            {SCALE_STEPS.map((step) => {
              const hex = resolvedTheme.scale[step];
              const textColor =
                contrastRatio("#FFFFFF", hex) >= contrastRatio("#14111C", hex)
                  ? "#FFFFFF"
                  : "#14111C";
              return (
                <div
                  key={step}
                  className="scale-cell"
                  style={{ backgroundColor: hex, color: textColor }}
                  title={hex}
                >
                  <span>{step}</span>
                </div>
              );
            })}
          </div>
        </section>

        <section className="settings-card">
          <div className="settings-row">
            <div className="card-title">{t("settings.advancedTitle")}</div>
            <div className="row-actions">
              <button
                type="button"
                className="btn btn-sm"
                onClick={handleExportTheme}
              >
                {t("settings.exportTheme")}
              </button>
              <button
                type="button"
                className="btn btn-sm"
                onClick={() => importRef.current?.click()}
              >
                {t("settings.importTheme")}
              </button>
              <button
                type="button"
                className="link-btn"
                onClick={resetAllOverrides}
              >
                {t("settings.resetOverrides")}
              </button>
            </div>
          </div>
          <input
            ref={importRef}
            type="file"
            accept=".mee-tema,.mee,.meetin-tema,.json"
            style={{ display: "none" }}
            onChange={(e) => void handleImportTheme(e)}
          />

          <div className="role-table">
            {THEME_ROLES.map((meta) => {
              const current = resolvedTheme.resolved[meta.key];
              const isManual = Boolean(config.overrides[meta.key]);
              const keys = ROLE_LABEL_KEYS[meta.key];
              const ratio = meta.contrastAgainst
                ? contrastRatio(
                    current,
                    resolvedTheme.resolved[meta.contrastAgainst],
                  )
                : null;
              const pass = ratio !== null && ratio >= 4.5;

              return (
                <div key={meta.key} className="role-row">
                  <div className="role-info">
                    <input
                      type="color"
                      className="role-swatch"
                      value={current}
                      onChange={(e) =>
                        setRoleOverride(meta.key, e.target.value)
                      }
                    />
                    <div>
                      <div className="role-name">
                        <span>{t(keys.name)}</span>
                        {isManual && (
                          <button
                            type="button"
                            className="manual-pill"
                            title={t("settings.revertRole")}
                            onClick={() => clearRoleOverride(meta.key)}
                          >
                            {t("settings.manualBadge")} ×
                          </button>
                        )}
                      </div>
                      <div className="muted small">{t(keys.desc)}</div>
                    </div>
                  </div>

                  <div className="role-right">
                    <input
                      className="role-hex-input"
                      value={current}
                      onChange={(e) => {
                        const v = e.target.value;
                        if (/^#?[0-9a-fA-F]{6}$/.test(v.trim())) {
                          setRoleOverride(meta.key, v);
                        }
                      }}
                    />
                    {ratio !== null ? (
                      <span
                        className={`contrast-pill ${pass ? "is-pass" : "is-fail"}`}
                      >
                        {pass ? "✓" : "!"} {ratio.toFixed(1)}:1
                      </span>
                    ) : (
                      <span className="contrast-pill is-neutral">—</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </section>

        <section className="settings-card">
          <div className="settings-row">
            <div>
              <div className="card-title">{t("settings.islandSurfaceTitle")}</div>
              <div className="muted small">
                {t("settings.islandSurfaceDesc")}
              </div>
            </div>
            <div className="segmented-control">
              {(["match_theme", "always_dark"] as IslandSurface[]).map((s) => (
                <button
                  key={s}
                  type="button"
                  className={`seg-btn ${config.islandSurface === s ? "is-active" : ""}`}
                  onClick={() => commitConfig({ ...config, islandSurface: s })}
                >
                  {t(`settings.islandSurface.${s}` as TranslationKey)}
                </button>
              ))}
            </div>
          </div>

          <div className="settings-row" style={{ marginTop: 12 }}>
            <div>
              <div className="card-title">{t("settings.opacityTitle")}</div>
              <div className="muted small">
                {Math.round(config.acrylicOpacity * 100)}%
              </div>
            </div>
            <input
              type="range"
              min={65}
              max={100}
              value={Math.round(config.acrylicOpacity * 100)}
              onChange={(e) =>
                commitConfig({
                  ...config,
                  acrylicOpacity: Number(e.target.value) / 100,
                })
              }
            />
          </div>

          <div className="settings-row" style={{ marginTop: 12 }}>
            <span className="muted small">{t("settings.resetHint")}</span>
            <button type="button" className="btn" onClick={resetDefaults}>
              {t("settings.resetDefaults")}
            </button>
          </div>
        </section>
      </div>

      <aside className="appearance-preview">
        <div className="preview-heading">{t("settings.livePreview")}</div>
        <div
          className="preview-stage"
          style={
            {
              ...(resolvedTheme.resolved as Record<string, string>),
              "--ada-zemin": resolvedTheme.adaZemin,
              "--ada-metin": resolvedTheme.adaMetin,
            } as React.CSSProperties
          }
        >
          <div className="preview-stage-top">
            <span className="preview-bubble-pill">{t("settings.previewBubble")}</span>
            <MeetinSheep anim="idle" size={44} />
          </div>

          <div className="preview-island-card">
            <div className="preview-search-box">
              <span>bülten</span>
            </div>
            <div className="preview-island-row is-selected">
              <span className="abbr-chip">BLT</span>
              <span>Bülten girişi</span>
            </div>
            <div className="preview-island-row">
              <span className="abbr-chip">STR</span>
              <span>Simultane raporu</span>
            </div>
            <div className="preview-island-row">
              <span className="abbr-chip">AKHA</span>
              <span>Birim adı</span>
            </div>
          </div>

          <div className="preview-toast-pill">
            ✓ {t("settings.previewSaved")}
          </div>
        </div>

        <div className="settings-card preview-components-card">
          <div className="preview-btn-row">
            <button type="button" className="btn btn-primary btn-sm">
              {t("settings.previewPrimaryBtn")}
            </button>
            <button type="button" className="btn btn-sm">
              {t("settings.previewSecondaryBtn")}
            </button>
            <span className="abbr-chip" style={{ marginLeft: "auto" }}>
              SYG
            </span>
          </div>
          <div className="preview-selected-row">
            {t("settings.previewSelectedRow")}
          </div>
          <div className="preview-status-row small">
            <span style={{ color: "var(--basari)" }}>● {t("settings.statusOk")}</span>
            <span style={{ color: "var(--uyari)" }}>● {t("settings.statusWarn")}</span>
            <span style={{ color: "var(--hata)" }}>● {t("settings.statusErr")}</span>
            <span className="muted">{t("settings.statusSecondary")}</span>
          </div>
        </div>
      </aside>
    </div>
  );
}
