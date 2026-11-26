import { describe, expect, it } from "vitest";
import type { PhraseRecord } from "../studio/types";
import { createPhraseSearchIndex } from "./searchEngine";

function makePhrase(
  id: number,
  abbr: string,
  content: string,
  description = "",
  tags: string[] = [],
  use_count = 0,
): PhraseRecord {
  return {
    id,
    folder_id: null,
    folder_ids: [],
    abbr,
    content,
    content_html: null,
    description,
    case_mode: "match_typed",
    shortcut: null,
    enabled: true,
    use_count,
    last_used_at: null,
    created_at: "2026-11-26 09:00:00",
    updated_at: "2026-11-26 09:00:00",
    deleted_at: null,
    tags,
  };
}

describe("searchEngine", () => {
  it("matches Turkish abbreviations, descriptions, content, and tags", () => {
    const idx = createPhraseSearchIndex([
      makePhrase(1, "MİPY", "Metin ile Pratik Yazım", "Üretkenlik notu", ["genel"], 5),
      makePhrase(2, "ŞĞÜ", "Şehirlerarası Güvenli Ürün", "Genel not", ["resmi"], 12),
      makePhrase(3, "SLM", "Selamlar herkese", "Kısa selam", ["sohbet"], 2),
    ]);

    expect(idx.search("")[0].abbr).toBe("ŞĞÜ");
    expect(idx.search("mipy")[0].id).toBe(1);
    expect(idx.search("Genel not")[0].id).toBe(2);
    expect(idx.search("sohbet")[0].id).toBe(3);
  });

  it("searches 10,000 phrases in under 30 ms", () => {
    const list: PhraseRecord[] = [];
    for (let i = 0; i < 10_000; i++) {
      list.push(
        makePhrase(
          i + 1,
          `KIS${i}`,
          `Örnek genişletilecek metin numarası ${i}`,
          `Açıklama ${i}`,
          ["test"],
          i % 10,
        ),
      );
    }

    const idx = createPhraseSearchIndex(list);
    const t0 = performance.now();
    const res = idx.search("KIS9876", 20);
    const elapsed = performance.now() - t0;

    expect(res.length).toBeGreaterThan(0);
    expect(res[0].abbr).toBe("KIS9876");
    expect(elapsed).toBeLessThan(30);
  });
});
