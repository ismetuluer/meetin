import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import settingsSource from "./SettingsView.tsx?raw";
import studioSource from "./StudioView.tsx?raw";
import { ExportDialog } from "./ExportDialog";

const noop = async () => {};

describe("tek dışa aktarma penceresi (R3.4)", () => {
  it("kapsamı yazar, üç biçimi tek seçimli sunar, .mee seçili başlar", () => {
    const html = renderToStaticMarkup(
      <ExportDialog
        request={{
          scope: "subset",
          ids: [1, 2, 3],
          fileName: "is",
          title: "İş",
        }}
        libraryCount={10}
        onExport={noop}
        onClose={() => {}}
      />,
    );
    expect(html).toContain('role="dialog"');
    expect(html).toContain("İş · 3 ifade");
    expect(html.match(/type="radio"/g)?.length).toBe(3);
    expect(html).toContain('checked="" value="meetin"');
    const lib = renderToStaticMarkup(
      <ExportDialog
        request={{ scope: "library" }}
        libraryCount={10}
        onExport={noop}
        onClose={() => {}}
      />,
    );
    expect(lib).toContain("Tüm kitaplık · 10 ifade");
  });

  it("menülerde biçim başına ayrı dışa aktarma girişi kalmaz", () => {
    for (const src of [studioSource, settingsSource]) {
      expect(src.length).toBeGreaterThan(1000);
      expect(src).not.toContain("studio.ctxExportMee");
      expect(src).not.toContain("studio.ctxExportCsv");
      expect(src).not.toContain('onExport("csv")');
    }
    expect(studioSource).not.toContain("handleExportSubset");
    expect(studioSource).not.toContain("showListExportMenu");
    expect(studioSource.match(/t\("export\.menuEntry"\)/g)?.length).toBe(2);
    expect(settingsSource).toContain('t("export.menuEntry")');
  });
});
