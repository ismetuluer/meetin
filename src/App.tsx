import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SelectionBubbleApp } from "./bubble/SelectionBubbleApp";
import {
  applyThemeToDocument,
  DEFAULT_THEME_CONFIG,
  resolveThemeTokens,
  type ThemeConfig,
  DEFAULT_ACCENT_HEX,
} from "./design/palette";
import { BasketballHoopApp } from "./games/BasketballHoopApp";
import { setLanguage, useLanguage } from "./i18n";
import { MascotIslandApp } from "./island/MascotIslandApp";
import { StudioView } from "./studio/StudioView";
import type { AppSettingsDto } from "./studio/types";

type WindowRole = "main" | "mascot" | "bubble" | "hoop";

interface SystemThemeInfo {
  dark: boolean;
  accent_hex: string;
}

interface WindowBackdropInfo {
  windows_build: number;
  is_windows_11: boolean;
  main_backdrop: string;
  island_backdrop: string;
  mica_active: boolean;
  acrylic_active: boolean;
}

function parseThemeConfig(raw: string | null): ThemeConfig {
  if (!raw) return DEFAULT_THEME_CONFIG;
  try {
    const parsed = JSON.parse(raw) as Partial<ThemeConfig>;
    return {
      ...DEFAULT_THEME_CONFIG,
      ...parsed,
      overrides: { ...(parsed.overrides ?? {}) },
    };
  } catch {
    return DEFAULT_THEME_CONFIG;
  }
}

export function App() {
  useLanguage();

  const [role] = useState<WindowRole>(() => {
    try {
      const label = getCurrentWebviewWindow().label;
      if (label === "mascot") return "mascot";
      if (label === "bubble") return "bubble";
      if (label === "hoop") return "hoop";
      return "main";
    } catch {
      return "main";
    }
  });

  useEffect(() => {
    const isTransparent =
      role === "mascot" || role === "bubble" || role === "hoop";
    document.documentElement.classList.toggle("is-mascot-window", isTransparent);
    document.body.classList.toggle("is-mascot-window", isTransparent);

    let cancelled = false;
    void invoke<WindowBackdropInfo>("get_window_backdrop_info")
      .then((info) => {
        if (cancelled) return;
        document.documentElement.classList.toggle(
          "has-mica-backdrop",
          role === "main" && Boolean(info?.mica_active),
        );
        document.documentElement.classList.toggle(
          "has-acrylic-backdrop",
          role === "mascot" && Boolean(info?.acrylic_active),
        );
      })
      .catch(() => {
        if (cancelled) return;
        document.documentElement.classList.remove(
          "has-mica-backdrop",
          "has-acrylic-backdrop",
        );
      });

    if (role !== "mascot") {
      const preventBrowserMenu = (e: MouseEvent) => {
        e.preventDefault();
      };
      window.addEventListener("contextmenu", preventBrowserMenu);
      return () => {
        cancelled = true;
        window.removeEventListener("contextmenu", preventBrowserMenu);
      };
    }
    return () => {
      cancelled = true;
    };
  }, [role]);

  useEffect(() => {
    void invoke<AppSettingsDto>("get_app_settings")
      .then((s) => {
        if (s?.language) {
          setLanguage(s.language);
        }
      })
      .catch(() => {});

    const unlistenLang = listen<string>("language-changed", (ev) => {
      setLanguage(ev.payload);
    });

    return () => {
      void unlistenLang.then((f) => f());
    };
  }, []);

  useEffect(() => {
    let config: ThemeConfig = DEFAULT_THEME_CONFIG;
    let sysTheme: SystemThemeInfo = {
      dark: window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false,
      accent_hex: DEFAULT_ACCENT_HEX,
    };

    const sync = () => {
      const resolved = resolveThemeTokens(
        config,
        sysTheme.dark,
        sysTheme.accent_hex,
      );
      applyThemeToDocument(resolved, config.fontSize ?? "md");
    };

    sync();

    Promise.all([
      invoke<SystemThemeInfo>("get_system_theme").catch(() => sysTheme),
      invoke<string | null>("get_theme_config").catch(() => null),
    ]).then(([sys, cfgJson]) => {
      sysTheme = sys;
      config = parseThemeConfig(cfgJson);
      sync();
    });

    const unlistenSys = listen<SystemThemeInfo>("system-theme-changed", (ev) => {
      sysTheme = ev.payload;
      sync();
    });
    const unlistenCfg = listen<string>("theme-config-changed", (ev) => {
      config = parseThemeConfig(ev.payload);
      sync();
    });

    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onMediaChange = (e: MediaQueryListEvent) => {
      sysTheme = { ...sysTheme, dark: e.matches };
      sync();
    };
    media?.addEventListener?.("change", onMediaChange);

    return () => {
      void unlistenSys.then((f) => f());
      void unlistenCfg.then((f) => f());
      media?.removeEventListener?.("change", onMediaChange);
    };
  }, []);

  if (role === "mascot") {
    return <MascotIslandApp />;
  }
  if (role === "bubble") {
    return <SelectionBubbleApp />;
  }
  if (role === "hoop") {
    return <BasketballHoopApp />;
  }
  return <StudioView />;
}

export default App;
