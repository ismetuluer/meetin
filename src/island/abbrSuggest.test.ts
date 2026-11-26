import { describe, expect, it } from "vitest";
import { normalizeTurkishAbbr, suggestAbbreviation } from "./abbrSuggest";

describe("abbrSuggest", () => {
  it("suggests Turkish initials matching spec example MİPY", () => {
    expect(suggestAbbreviation("Metin ile Pratik Yazım")).toBe("MİPY");
    expect(suggestAbbreviation("iyi başlangıç bilgisi")).toBe("İBB");
    expect(suggestAbbreviation("ılık şafak mevsimi")).toBe("IŞM");
  });

  it("handles single words, punctuation, and empty text", () => {
    expect(suggestAbbreviation("Merhaba!")).toBe("MERH");
    expect(suggestAbbreviation("   ")).toBe("");
    expect(suggestAbbreviation("Bir İki Üç Dört Beş Altı Yedi")).toBe("BİÜDBA");
  });

  it("supports first_word, last_word, and tag suggestion modes", () => {
    expect(suggestAbbreviation("İlk Çalışma Notları", "first_word")).toBe(
      "İLK",
    );
    expect(suggestAbbreviation("İlk Çalışma Notları", "last_word")).toBe(
      "NOTLARI",
    );
    expect(
      suggestAbbreviation("Resmî yazı şablonu", "tag", ["#resmi", "genel"]),
    ).toBe("RESMİ");
    expect(suggestAbbreviation("Resmî yazı şablonu", "tag", [])).toBe("RYŞ");
  });

  it("normalizes dotted/dotless i separately for conflict detection", () => {
    expect(normalizeTurkishAbbr("MİPY")).toBe("mipy");
    expect(normalizeTurkishAbbr("MIPY")).toBe("mıpy");
    expect(normalizeTurkishAbbr("MİPY")).not.toBe(normalizeTurkishAbbr("MIPY"));
  });
});
