import { describe, expect, it } from "vitest";
import { t, type MetinAnahtari } from ".";

describe("t()", () => {
  it("var olan anahtarı döndürür", () => {
    expect(t("app.name")).toBe("Meetin");
  });
  it("eksik anahtarda hata fırlatır", () => {
    expect(() => t("yok" as MetinAnahtari)).toThrow(/Eksik metin/);
  });
});
