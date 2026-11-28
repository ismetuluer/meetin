/**
 * İfade listesinde Windows Gezgini gibi seçim (R3.1).
 * - Tık: o satırı açar, çoklu seçimi bırakır, çapa olur.
 * - Ctrl+tık: satırı seçime ekler/çıkarır (açık satır seçimin başlangıcıdır).
 * - Shift+tık: çapadan tıklanan satıra kadar aralığı seçer; Ctrl+Shift aralığı ekler.
 */
export interface ListSelection {
  /** Çoklu seçim; boşken yalnız açık satır seçilidir. */
  ids: number[];
  /** Shift aralığının başladığı satır */
  anchor: number | null;
}

export interface ClickMods {
  ctrl: boolean;
  shift: boolean;
}

export const EMPTY_SELECTION: ListSelection = { ids: [], anchor: null };

function inOrder(order: number[], ids: Iterable<number>): number[] {
  const set = new Set(ids);
  return order.filter((id) => set.has(id));
}

export function clickRow(
  sel: ListSelection,
  order: number[],
  id: number,
  mods: ClickMods,
  openId: number | null,
): { sel: ListSelection; open: boolean } {
  if (!mods.ctrl && !mods.shift) {
    return { sel: { ids: [], anchor: id }, open: true };
  }
  const base =
    sel.ids.length > 0
      ? sel.ids
      : openId !== null && order.includes(openId)
        ? [openId]
        : [];
  if (!mods.shift) {
    const next = base.includes(id)
      ? base.filter((x) => x !== id)
      : [...base, id];
    return { sel: { ids: inOrder(order, next), anchor: id }, open: false };
  }
  const anchor =
    sel.anchor !== null && order.includes(sel.anchor)
      ? sel.anchor
      : (openId ?? id);
  const a = order.indexOf(anchor);
  const b = order.indexOf(id);
  const range =
    a < 0 || b < 0 ? [id] : order.slice(Math.min(a, b), Math.max(a, b) + 1);
  const ids = mods.ctrl ? inOrder(order, [...base, ...range]) : range;
  return { sel: { ids, anchor: a < 0 ? id : anchor }, open: false };
}

/** Sağ tık ya da sürükleme hangi ifadelere uygulanır: seçili bir satırdaysa bütün seçime. */
export function actionIds(
  sel: ListSelection,
  order: number[],
  id: number,
): number[] {
  return sel.ids.length > 1 && sel.ids.includes(id)
    ? inOrder(order, sel.ids)
    : [id];
}

/** Liste değişince (filtre, silme) görünmeyen satırlar seçimden düşer. */
export function pruneSelection(
  sel: ListSelection,
  order: number[],
): ListSelection {
  const ids = inOrder(order, sel.ids);
  const anchor =
    sel.anchor !== null && order.includes(sel.anchor) ? sel.anchor : null;
  return ids.length === sel.ids.length && anchor === sel.anchor
    ? sel
    : { ids, anchor };
}
