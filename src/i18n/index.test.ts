import { describe, expect, it } from "vitest";
import { t, type TranslationKey } from "./index";
import tr from "./tr.json";

describe("i18n t()", () => {
  it("returns translated Turkish strings for all keys", () => {
    for (const key of Object.keys(tr) as TranslationKey[]) {
      const value = t(key);
      expect(value).toBeTruthy();
      expect(value).not.toBe(key);
    }
  });

  it("interpolates template parameters", () => {
    expect(t("status.phraseCount", { count: 42 })).toBe("42 etkin ifade");
    expect(t("error.generic", { message: "test" })).toBe("Bir hata oluştu: test");
  });
});
