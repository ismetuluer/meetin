import { describe, expect, it } from "vitest";
import { normalizeTurkishAbbr, suggestAbbreviation } from "./abbrSuggest";

describe("abbrSuggest", () => {
  it("suggests Turkish initials matching spec example CİPY", () => {
    expect(suggestAbbreviation("Claude ile Proje Yapmak")).toBe("CİPY");
    expect(suggestAbbreviation("istanbul büyükşehir belediyesi")).toBe("İBB");
    expect(suggestAbbreviation("ığdır şube müdürlüğü")).toBe("IŞM");
  });

  it("handles single words, punctuation, and empty text", () => {
    expect(suggestAbbreviation("Merhaba!")).toBe("MERH");
    expect(suggestAbbreviation("   ")).toBe("");
    expect(suggestAbbreviation("Bir İki Üç Dört Beş Altı Yedi")).toBe("BİÜDBA");
  });

  it("supports first_word, last_word, and tag suggestion modes", () => {
    expect(suggestAbbreviation("İstanbul Büyükşehir Belediyesi", "first_word")).toBe(
      "İSTANBUL",
    );
    expect(suggestAbbreviation("İstanbul Büyükşehir Belediyesi", "last_word")).toBe(
      "BELEDİYE",
    );
    expect(
      suggestAbbreviation("Resmî yazı şablonu", "tag", ["#resmi", "haber"]),
    ).toBe("RESMİ");
    expect(suggestAbbreviation("Resmî yazı şablonu", "tag", [])).toBe("RYŞ");
  });

  it("normalizes dotted/dotless i separately for conflict detection", () => {
    expect(normalizeTurkishAbbr("CİPY")).toBe("cipy");
    expect(normalizeTurkishAbbr("CIPY")).toBe("cıpy");
    expect(normalizeTurkishAbbr("CİPY")).not.toBe(normalizeTurkishAbbr("CIPY"));
  });
});
