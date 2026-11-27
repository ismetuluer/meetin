import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import css from "./stackPanel.css?raw";
import { StackPanel } from "./StackPanel";

const base = {
  title: "Geçmiş",
  closeLabel: "Geçmişi kapat",
  emptyText: "Henüz sürüm yok",
  actionLabel: "Geri yükle",
  onAction: () => {},
  onClose: () => {},
};

const items = [
  { id: 3, meta: "bugün · 14:32", preview: "en yeni" },
  { id: 2, meta: "dün · 09:10", preview: "orta" },
  { id: 1, meta: "3 gün önce · 18:00", preview: "eski" },
  { id: 0, meta: "geçen hafta · 08:00", preview: "en eski" },
];

describe("StackPanel (R3.0d)", () => {
  it("kapalıyken hiçbir şey çizmez", () => {
    expect(renderToStaticMarkup(<StackPanel {...base} open={false} items={items} />)).toBe("");
  });

  it("ilk karede yığılı başlar: kartlar etkileşime kapalı, en fazla iki kart arkada görünür", () => {
    const html = renderToStaticMarkup(<StackPanel {...base} open items={items} />);
    expect(html).not.toContain("is-fanned");
    expect(html.match(/inert=""/g)?.length).toBe(items.length);
    expect(html).toContain("translateY(14px) scale(0.92)");
    expect(html).toContain("opacity:0");
    expect(html).toContain('aria-label="Geçmişi kapat"');
    expect(html).toContain("bugün · 14:32");
  });

  it("sürüm yoksa boş mesajı gösterir", () => {
    const html = renderToStaticMarkup(<StackPanel {...base} open items={[]} />);
    expect(html).toContain("Henüz sürüm yok");
    expect(html).not.toContain("meetin-stack-card");
  });

  it("ortak yayı ve yalnız §11.4 renk rollerini kullanır", () => {
    expect(css).toContain("transform var(--yay-sure) var(--yay-egri)");
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(/i);
  });
});
