import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import helpSource from "../studio/HelpView.tsx?raw";
import settingsSource from "../studio/SettingsView.tsx?raw";
import css from "./kbd.css?raw";
import { KbdGroup, ShortcutField, ShortcutText } from "./Kbd";

// R4.2: StudioView.tsx bileşenlere bölündü; denetim bütün stüdyo kabuğu kaynağında yapılır.
const studioFiles = import.meta.glob<string>(
  ["../studio/Studio*.tsx", "../studio/studio*.ts", "../studio/useStudio*.ts", "!../studio/*.test.*"],
  { query: "?raw", import: "default", eager: true },
);
const studioSource = Object.values(studioFiles).join("\n");

describe("Kbd (R3.0e)", () => {
  it("kısayolu ayrı tuş kutucuklarına böler", () => {
    const html = renderToStaticMarkup(<KbdGroup shortcut="Ctrl+Alt+Space" />);
    expect(html.match(/<kbd/g)?.length).toBe(3);
    expect(html).toContain(">Ctrl</kbd>");
    expect(html).toContain(">Alt</kbd>");
    expect(html).toContain(">Boşluk</kbd>");
  });

  it("metin içindeki kısayolları kutucuğa çevirir, kalan yazıyı korur", () => {
    const html = renderToStaticMarkup(<ShortcutText text="Ctrl+K ile paleti açın." />);
    expect(html).toContain(">K</kbd>");
    expect(html).toContain(" ile paleti açın.");
  });

  it("kısayol alanı adlandırılmış bir düğmedir; boşken yer tutucuyu gösterir", () => {
    const filled = renderToStaticMarkup(
      <ShortcutField value="Ctrl+Alt+P" label="Duraklat" title="ipucu" placeholder="yok" onKeyDown={() => {}} />,
    );
    expect(filled).toContain('type="button"');
    expect(filled).toContain('aria-label="Duraklat: Ctrl+Alt+P"');
    expect(filled).toContain(">P</kbd>");
    const empty = renderToStaticMarkup(
      <ShortcutField value="" label="Duraklat" title="ipucu" placeholder="yok" onKeyDown={() => {}} />,
    );
    expect(empty).toContain(">yok</span>");
    expect(empty).not.toContain("<kbd");
  });

  it("yalnız §11.4 renk rollerini kullanır", () => {
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(/i);
  });

  it("arayüzde düz yazı kısayol rozeti ve salt okunur kısayol kutusu kalmaz", () => {
    expect(Object.keys(studioFiles).length).toBeGreaterThanOrEqual(14);
    for (const src of [helpSource, settingsSource, studioSource]) {
      expect(src.length).toBeGreaterThan(1000);
      expect(src).not.toContain("kbd-badge");
      expect(src).not.toMatch(/<input[^>]*readOnly[^>]*\n[^<]*shortcut/);
    }
    expect(settingsSource).toContain('if (e.key === "Tab") return;');
  });
});
