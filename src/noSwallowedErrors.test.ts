import { describe, expect, it } from "vitest";

const sources = import.meta.glob(
  ["/src/**/*.{ts,tsx}", "!/src/**/*.test.*", "!/src/zz-*"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
) as Record<string, string>;

describe("hatalar sessizce yutulmaz (AGENTS.md)", () => {
  it("gerekçesiz boş `.catch(() => {})` kalmaz", () => {
    const files = Object.keys(sources);
    expect(files.length).toBeGreaterThan(50);
    const offenders = files.filter((f) =>
      /\.catch\(\(\) => \{\}\)/.test(sources[f]),
    );
    // Yalnız günlük yazımının kendisi: günlük yazılamazsa yapılacak başka bir şey yoktur
    expect(offenders).toEqual([
      "/src/errorReporting.ts",
    ]);
  });
});
