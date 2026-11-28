import { useEffect, type Dispatch, type SetStateAction } from "react";
import type { ListSelection } from "./listSelection";
import { isEditableTarget, type PageMode } from "./studioHelpers";

interface StudioShortcutOptions {
  pageMode: PageMode;
  showPalette: boolean;
  showOnboarding: boolean;
  listSelectable: boolean;
  displayedOrder: number[];
  selection: ListSelection;
  setShowPalette: Dispatch<SetStateAction<boolean>>;
  setPageMode: Dispatch<SetStateAction<PageMode>>;
  setSelection: Dispatch<SetStateAction<ListSelection>>;
}

/** Stüdyo pencere kısayolları: Ctrl+K palet, F1 yardım, Esc geri; liste için Ctrl+A ve Esc (R3.1). */
export function useStudioShortcuts({
  pageMode,
  showPalette,
  showOnboarding,
  listSelectable,
  displayedOrder,
  selection,
  setShowPalette,
  setPageMode,
  setSelection,
}: StudioShortcutOptions) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setShowPalette((prev) => !prev);
      } else if (e.key === "F1") {
        e.preventDefault();
        setPageMode((m) => (m === "help" ? "studio" : "help"));
      } else if (
        e.key === "Escape" &&
        pageMode === "help" &&
        !showPalette &&
        !showOnboarding
      ) {
        e.preventDefault();
        setPageMode("studio");
      } else if (pageMode === "studio" && !showPalette && !isEditableTarget(e.target)) {
        // Liste seçimi (R3.1): Ctrl+A hepsini seçer, Esc seçimi bırakır
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a" && listSelectable) {
          e.preventDefault();
          setSelection((prev) => ({ ...prev, ids: displayedOrder }));
        } else if (e.key === "Escape" && selection.ids.length > 0) {
          e.preventDefault();
          setSelection((prev) => ({ ...prev, ids: [] }));
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    pageMode,
    showPalette,
    showOnboarding,
    listSelectable,
    displayedOrder,
    selection.ids.length,
    setShowPalette,
    setPageMode,
    setSelection,
  ]);
}
