import { describe, expect, it } from "vitest";
import tr from "../i18n/tr.json";
import studioCss from "./studio.css?raw";
import studioSource from "./StudioView.tsx?raw";

describe("düzenleyici düzeni (R3.2)", () => {
  it("klasör seçimi kısayol kutusundan ayrı bir alandır (A2)", () => {
    expect(studioSource.length).toBeGreaterThan(1000);
    const shortcutBox = studioSource.indexOf("<ShortcutField");
    const folderGroup = studioSource.indexOf(
      'className="meta-field-group folder-field-group"',
    );
    expect(shortcutBox).toBeGreaterThan(0);
    expect(folderGroup).toBeGreaterThan(shortcutBox);
    // Kısayol kutusu ile klasör alanı arasında başka bir kutu kapanışı olmalı
    expect(studioSource.slice(shortcutBox, folderGroup)).not.toContain(
      "<select",
    );
    expect(studioSource).not.toContain("inline-case-select");
  });

  it("kural haplarında Yalnız/Hariç türü yazılır (A3)", () => {
    expect(studioSource).toContain(
      't("studio.ruleKindExclude") : t("studio.ruleKindOnly")',
    );
    expect(tr["studio.ruleKindOnly"]).toBe("Yalnız");
    expect(tr["studio.ruleKindExclude"]).toBe("Hariç");
  });

  it("işlem bildirimi düzeni kaydırmayan kısa süreli bildirimdir, hata kapatılabilir", () => {
    expect(studioSource).toContain('className="studio-toast"');
    expect(studioSource).not.toContain(
      '<div className="banner-inline is-ok">{notice}</div>',
    );
    expect(studioSource).toContain(
      "setTimeout(() => setNotice(null), NOTICE_MS)",
    );
    expect(studioCss).toMatch(/\.studio-toast-region \{[^}]*position: fixed/);
    expect(studioSource).toContain('aria-label={t("studio.dismiss")}');
  });

  it("çalışan uygulama listesi hatası yutulmaz", () => {
    expect(studioSource).not.toMatch(
      /list_running_apps"\)\s*\.then\(setRunningApps\)\s*\.catch\(\(\) => \{\}\)/,
    );
  });
});
