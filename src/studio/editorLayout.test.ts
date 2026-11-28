import { describe, expect, it } from "vitest";
import tr from "../i18n/tr.json";
import studioCss from "./studio.css?raw";

// R4.2: StudioView.tsx bileşenlere bölündü; denetim bütün stüdyo kabuğu kaynağında yapılır.
const studioFiles = import.meta.glob<string>(
  ["./Studio*.tsx", "./studio*.ts", "./useStudio*.ts", "!./*.test.*"],
  { query: "?raw", import: "default", eager: true },
);
const studioSource = Object.values(studioFiles).join("\n");

describe("düzenleyici düzeni (R3.2)", () => {
  it("klasör seçimi kısayol kutusundan ayrı bir alandır (A2)", () => {
    expect(Object.keys(studioFiles).length).toBeGreaterThanOrEqual(14);
    expect(studioSource.length).toBeGreaterThan(1000);
    // Sıra denetimi, kısayol ve klasör alanını birlikte taşıyan dosyada yapılır
    const fieldsSource = studioFiles["./StudioEditorFields.tsx"] ?? "";
    expect(fieldsSource.length).toBeGreaterThan(1000);
    expect(studioSource.split("<ShortcutField").length - 1).toBe(1);
    const shortcutBox = fieldsSource.indexOf("<ShortcutField");
    const folderGroup = fieldsSource.indexOf(
      'className="meta-field-group folder-field-group"',
    );
    expect(shortcutBox).toBeGreaterThan(0);
    expect(folderGroup).toBeGreaterThan(shortcutBox);
    // Kısayol kutusu ile klasör alanı arasında başka bir kutu kapanışı olmalı
    expect(fieldsSource.slice(shortcutBox, folderGroup)).not.toContain(
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
