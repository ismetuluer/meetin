import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import settingsSource from "../studio/SettingsView.tsx?raw";
import toggleCss from "./toggle.css?raw";
import { Toggle } from "./Toggle";

describe("Toggle (R3.0a)", () => {
  it("yerel onay kutusu üzerine kurulur: klavye, etiket ve ekran okuyucu çalışır", () => {
    const html = renderToStaticMarkup(<Toggle checked readOnly aria-label="Pano geçmişi" />);
    expect(html).toContain('type="checkbox"');
    expect(html).toContain('role="switch"');
    expect(html).toContain('aria-label="Pano geçmişi"');
    expect(html).toContain("checked");
    expect(html).toContain('class="meetin-toggle"');
  });

  it("ek sınıfı ve pasif durumu korur", () => {
    const html = renderToStaticMarkup(<Toggle className="ek" disabled readOnly />);
    expect(html).toContain('class="meetin-toggle ek"');
    expect(html).toContain("disabled");
  });

  it("ortak yayı ve yalnız §11.4 renk rollerini kullanır", () => {
    expect(toggleCss).toContain("var(--yay-sure-kisa) var(--yay-egri)");
    expect(toggleCss).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(/i);
  });

  it("Ayarlar'da düz onay kutusu kalmaz", () => {
    expect(settingsSource).toContain("<Toggle");
    expect(settingsSource).not.toContain('type="checkbox"');
  });
});
