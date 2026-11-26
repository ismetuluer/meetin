import { describe, expect, it } from "vitest";
import { getHelpSections, HELP_SECTIONS, searchHelpSections } from "./helpContent";

describe("helpContent", () => {
  it("contains 10 unique, non-empty sections in both tr and en", () => {
    for (const sections of [HELP_SECTIONS, getHelpSections("tr"), getHelpSections("en")]) {
      expect(sections).toHaveLength(10);
      const ids = new Set(sections.map((s) => s.id));
      expect(ids.size).toBe(10);
      for (const sec of sections) {
        expect(sec.title.trim().length).toBeGreaterThan(0);
        expect(sec.summary.trim().length).toBeGreaterThan(0);
        expect(sec.blocks.length).toBeGreaterThan(0);
      }
    }
    expect(getHelpSections("en")[0].title).toBe("Quick Start");
    expect(getHelpSections("tr")[0].title).toBe("Hızlı Başlangıç");
  });

  it("returns all sections for empty search", () => {
    expect(searchHelpSections("")).toHaveLength(10);
    expect(searchHelpSections("   ")).toHaveLength(10);
    expect(searchHelpSections("", "en")).toHaveLength(10);
  });

  it("searches with Turkish case-insensitive folding", () => {
    expect(searchHelpSections("İML", "tr").some((s) => s.id === "variables")).toBe(
      true,
    );
    expect(searchHelpSections("TARİH", "tr").some((s) => s.id === "variables")).toBe(
      true,
    );
    expect(
      searchHelpSections("shift+f3", "tr").some((s) => s.id === "turkish-case"),
    ).toBe(true);
    expect(searchHelpSections("klasör", "tr").some((s) => s.id === "library")).toBe(
      true,
    );
    expect(searchHelpSections("smartscreen", "tr").some((s) => s.id === "faq")).toBe(
      true,
    );
  });

  it("searches English help sections accurately", () => {
    expect(searchHelpSections("wysiwyg", "en").some((s) => s.id === "variables")).toBe(
      true,
    );
    expect(searchHelpSections("clipboard", "en").some((s) => s.id === "clipboard")).toBe(
      true,
    );
    expect(searchHelpSections("password", "en").some((s) => s.id === "faq")).toBe(
      true,
    );
  });
});
