import { describe, expect, it } from "vitest";
import { normalizeAbbr, uniqueCopyAbbr } from "./phraseCopy";

describe("ifade kopyası kısaltması (R3.1a)", () => {
  it("ilk boş sayıyı ekler", () => {
    expect(uniqueCopyAbbr("mipy", ["mipy"], false)).toBe("mipy2");
    expect(uniqueCopyAbbr("mipy", ["mipy", "MİPY2"], false)).toBe("mipy3");
  });

  it("sondaki sayıyı artırır, yalnız sayıdan oluşanı korur", () => {
    expect(uniqueCopyAbbr("ad2", ["ad2", "ad3"], false)).toBe("ad4");
    expect(uniqueCopyAbbr("42", ["42"], false)).toBe("422");
  });

  it("tr-TR büyük/küçük harf ve aksan katlamasını Rust ile aynı uygular", () => {
    expect(normalizeAbbr("İSİM", false)).toBe("isim");
    expect(normalizeAbbr("IŞIK", false)).toBe("ışık");
    expect(normalizeAbbr("Işık", true)).toBe("isik");
    expect(uniqueCopyAbbr("sik", ["sik", "şık2"], true)).toBe("sik3");
    expect(uniqueCopyAbbr("sik", ["sik", "şık2"], false)).toBe("sik2");
  });

  it("boş kısaltmada boş döner", () => {
    expect(uniqueCopyAbbr("  ", [], false)).toBe("");
  });
});
