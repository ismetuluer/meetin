import { describe, expect, it } from "vitest";
import {
  actionIds,
  clickRow,
  EMPTY_SELECTION,
  pruneSelection,
} from "./listSelection";
import studioSource from "./StudioView.tsx?raw";
import studioCss from "./studio.css?raw";

const order = [10, 20, 30, 40, 50];
const none = { ctrl: false, shift: false };
const ctrl = { ctrl: true, shift: false };
const shift = { ctrl: false, shift: true };
const both = { ctrl: true, shift: true };

describe("liste seçimi (R3.1)", () => {
  it("düz tık satırı açar, çoklu seçimi bırakır ve çapa olur", () => {
    const r = clickRow({ ids: [10, 20], anchor: 10 }, order, 30, none, 10);
    expect(r).toEqual({ sel: { ids: [], anchor: 30 }, open: true });
  });

  it("Ctrl+tık açık satırdan başlayıp ekler, ikinci Ctrl+tık çıkarır", () => {
    const a = clickRow({ ids: [], anchor: 20 }, order, 40, ctrl, 20);
    expect(a).toEqual({ sel: { ids: [20, 40], anchor: 40 }, open: false });
    const b = clickRow(a.sel, order, 20, ctrl, 20);
    expect(b.sel.ids).toEqual([40]);
    expect(clickRow(EMPTY_SELECTION, order, 30, ctrl, null).sel.ids).toEqual([
      30,
    ]);
  });

  it("Shift+tık çapadan aralık seçer, çapa yerinde kalır; Ctrl+Shift ekler", () => {
    const a = clickRow({ ids: [], anchor: 20 }, order, 40, shift, 20);
    expect(a.sel).toEqual({ ids: [20, 30, 40], anchor: 20 });
    const up = clickRow(a.sel, order, 10, shift, 20);
    expect(up.sel.ids).toEqual([10, 20]);
    const add = clickRow({ ids: [50], anchor: 50 }, order, 30, both, 50);
    expect(add.sel.ids).toEqual([30, 40, 50]);
  });

  it("çapa listede yoksa (filtre) aralık tıklanan satırdan başlar", () => {
    const r = clickRow({ ids: [], anchor: 99 }, order, 30, shift, null);
    expect(r.sel).toEqual({ ids: [30], anchor: 30 });
  });

  it("sağ tık seçili satırdaysa bütün seçime, değilse yalnız o satıra uygulanır", () => {
    const sel = { ids: [40, 20], anchor: 20 };
    expect(actionIds(sel, order, 40)).toEqual([20, 40]);
    expect(actionIds(sel, order, 30)).toEqual([30]);
    expect(actionIds({ ids: [20], anchor: 20 }, order, 20)).toEqual([20]);
  });

  it("görünmeyen satırlar seçimden düşer", () => {
    const sel = { ids: [20, 30], anchor: 30 };
    expect(pruneSelection(sel, [10, 20])).toEqual({ ids: [20], anchor: null });
    expect(pruneSelection(sel, order)).toBe(sel);
  });
});

describe("ifade listesi arayüzü (R3.1)", () => {
  it("satırlarda onay kutusu ve anlamsız kaydırma oku kalmaz, toplu işlemler sağ tıkta", () => {
    expect(studioSource.length).toBeGreaterThan(1000);
    expect(studioSource).not.toContain("row-select-box");
    expect(studioSource).not.toContain("row-swipe-trigger");
    expect(studioCss).not.toContain("row-select-box");
    expect(studioSource).toContain("handleRowClick(p, e)");
    expect(studioSource).toContain(
      "ids: actionIds(selection, displayedOrder, p.id)",
    );
    expect(studioSource).toContain('title={t("studio.rowHint")}');
    expect(studioSource).not.toContain("studio.bulkMoveTo");
  });
});
