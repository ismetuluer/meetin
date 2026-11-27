import { describe, expect, it } from "vitest";
import { addTags, normalizeTag, parseTags } from "./tagLogic";

describe("etiket mantığı (R3.0b)", () => {
  it("baştaki # ve boşlukları atar", () => {
    expect(normalizeTag("  #resmi ")).toBe("resmi");
    expect(normalizeTag("##iş")).toBe("iş");
    expect(normalizeTag("  # ")).toBe("");
  });

  it("virgülle ayrılmış metinden birden çok etiket çıkarır, boşları atlar", () => {
    expect(parseTags("a, b,,  #c , ")).toEqual(["a", "b", "c"]);
    expect(parseTags("")).toEqual([]);
  });

  it("Türkçe büyük/küçük harfe duyarsız tekrar eklemez, ilk yazılışı korur", () => {
    expect(addTags(["İş"], "iş")).toEqual({ tags: ["İş"], added: [] });
    expect(addTags(["ışık"], "IŞIK")).toEqual({ tags: ["ışık"], added: [] });
    // tr-TR olmadan "I" → "i" eşlenir ve yanlışlıkla "ışık" ile "işik" ayrı kalırdı
    expect(addTags(["işik"], "IŞIK").added).toEqual(["IŞIK"]);
  });

  it("aynı girdideki tekrarları da birleştirir ve eklenenleri bildirir", () => {
    expect(addTags(["a"], "b, B, a, c")).toEqual({ tags: ["a", "b", "c"], added: ["b", "c"] });
  });

  it("girdi boşsa dizi aynı kalır", () => {
    const tags = ["a"];
    expect(addTags(tags, " , ").tags).toBe(tags);
  });
});
