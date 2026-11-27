import { describe, expect, it } from "vitest";
import { keyLabel, parseShortcut, splitShortcutText } from "./shortcutKeys";

describe("kısayol ayrıştırma (R3.0e)", () => {
  it("artı ve boşlukla yazılmış kısayolları tuşlara ayırır", () => {
    expect(parseShortcut("Ctrl+Alt+P")).toEqual(["Ctrl", "Alt", "P"]);
    expect(parseShortcut("Shift + F3")).toEqual(["Shift", "F3"]);
    expect(parseShortcut("F1")).toEqual(["F1"]);
    expect(parseShortcut("  ")).toEqual([]);
  });

  it("artı tuşunun kendisini kaybetmez", () => {
    expect(parseShortcut("Ctrl++")).toEqual(["Ctrl", "+"]);
  });

  it("Windows adlarını kullanır", () => {
    const tr = { space: "Boşluk" };
    expect(keyLabel("Super", tr)).toBe("Win");
    expect(keyLabel("Meta", tr)).toBe("Win");
    expect(keyLabel("Space", tr)).toBe("Boşluk");
    expect(keyLabel("Enter", tr)).toBe("Enter ↵");
    expect(keyLabel("Tab", tr)).toBe("Tab ⇥");
    expect(keyLabel("Escape", tr)).toBe("Esc");
    expect(keyLabel("ArrowUp", tr)).toBe("↑");
    expect(keyLabel("Left", tr)).toBe("←");
    expect(keyLabel("Control", tr)).toBe("Ctrl");
    expect(keyLabel("p", tr)).toBe("P");
  });

  it("metindeki kısayolları bulur, Türkçe sözcüklerle birleşenleri ve yalın Ctrl'yi atlar", () => {
    expect(splitShortcutText("Ctrl+K ile paleti, F1 ile yardımı açın.")).toEqual([
      { keys: "Ctrl+K" },
      { text: " ile paleti, " },
      { keys: "F1" },
      { text: " ile yardımı açın." },
    ]);
    expect(splitShortcutText("Ctrl+tıkla ya da Ctrl+bırak")).toEqual([{ text: "Ctrl+tıkla ya da Ctrl+bırak" }]);
    expect(splitShortcutText("Shift+F3 ile, Ctrl+Alt+Space ile")).toEqual([
      { keys: "Shift+F3" },
      { text: " ile, " },
      { keys: "Ctrl+Alt+Space" },
      { text: " ile" },
    ]);
    expect(splitShortcutText("F12 değil F13")).toEqual([{ keys: "F12" }, { text: " değil F13" }]);
  });
});
