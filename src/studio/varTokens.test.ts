import { describe, expect, it } from "vitest";
import editorSource from "./RichEditor.tsx?raw";
import { syncEditorHtml, toEditorInitialHtml } from "./richHtml";
import {
  pillifyHtml,
  previewText,
  splitVarSegments,
  trailingVarToken,
  unpillHtml,
  varLabel,
} from "./varTokens";

// R4.2: StudioView.tsx bileşenlere bölündü; denetim bütün stüdyo kabuğu kaynağında yapılır.
const studioFiles = import.meta.glob<string>(
  ["./Studio*.tsx", "./studio*.ts", "./useStudio*.ts", "!./*.test.*"],
  { query: "?raw", import: "default", eager: true },
);
const studioSource = Object.values(studioFiles).join("\n");

describe("değişken hapları (R3.3)", () => {
  it("Rust sözdizimindeki değişkenleri tanır, diğer {…} yazıları düz kalır", () => {
    expect(varLabel("{tarih}")).toBe("Tarih");
    expect(varLabel("{tarih:uzun}")).toBe("Tarih (uzun)");
    expect(varLabel("{tarih:+3g}")).toBe("Tarih +3g");
    expect(varLabel("{saat}")).toBe("Saat");
    expect(varLabel("{pano}")).toBe("Pano");
    expect(varLabel("{imleç}")).toBe("İmleç");
    expect(varLabel("{alan:Ad Soyad|Bay,Bayan}")).toBe("Alan: Ad Soyad");
    expect(varLabel("{ifade:İMZA}")).toBe("İfade: İMZA");
    expect(varLabel("{tarih:yarın}")).toBeNull();
    expect(varLabel("{bilinmeyen}")).toBeNull();
    expect(varLabel("{alan:}")).toBeNull();
  });

  it("metni parçalara ayırır ve okunur önizleme verir", () => {
    expect(splitVarSegments("Merhaba {saat} {x}")).toEqual([
      { text: "Merhaba " },
      { token: "{saat}", label: "Saat" },
      { text: " {x}" },
    ]);
    expect(previewText("Bugün {tarih:uzun}\n  saat {saat}")).toBe(
      "Bugün Tarih (uzun) saat Saat",
    );
  });

  it("hap HTML'si ham değişkene birebir geri döner (gidiş-dönüş)", () => {
    const samples = [
      "Merhaba {tarih:uzun} tarihinde {saat}",
      "a < b & {pano} > c",
      '{alan:Ad "Soyad"} ve {ifade:İMZA}',
      "Değişkensiz metin",
    ];
    for (const text of samples) {
      const html = toEditorInitialHtml(text, null);
      const pilled = pillifyHtml(html);
      expect(syncEditorHtml(unpillHtml(pilled)).content).toBe(text);
    }
    const rich = "<b>Sayın {alan:Ad}</b>, <i>{tarih}</i>";
    const back = syncEditorHtml(unpillHtml(pillifyHtml(rich)));
    expect(back.content_html).toBe(rich);
    expect(back.content).toBe("Sayın {alan:Ad}, {tarih}");
  });

  it("hap düzenlenemez, ham değeri öznitelikte taşır ve etiket içine dokunmaz", () => {
    const pilled = pillifyHtml('<a href="https://x.test/{saat}">{saat}</a>');
    expect(pilled).toContain('href="https://x.test/{saat}"');
    expect(pilled).toContain(
      'contenteditable="false" data-var="{saat}">Saat</span>',
    );
    expect(pillifyHtml('{alan:A"B}')).toContain('data-var="{alan:A&quot;B}"');
  });

  it("yazarken imleçten önce tamamlanan değişkeni bulur", () => {
    expect(trailingVarToken("Saat şu an {saat}")).toBe("{saat}");
    expect(trailingVarToken("Saat şu an {saat")).toBeNull();
    expect(trailingVarToken("{bilinmeyen}")).toBeNull();
  });
});

describe("düzenleyici ve liste hap kullanımı (R3.3)", () => {
  it("düzenleyici hapla yükler, kayıttan ve kopyalamadan önce ham yazıya çevirir", () => {
    expect(editorSource.length).toBeGreaterThan(1000);
    expect(editorSource).toContain("pillifyHtml(toEditorInitialHtml(content, contentHtml))");
    expect(editorSource).toContain("syncEditorHtml(unpillHtml(root.innerHTML))");
    expect(editorSource).toContain("sanitizeRichHtml(unpillHtml(holder.innerHTML))");
  });

  it("ayrı önizleme şeridi ve köşeli parantezli önizleme kalmaz", () => {
    expect(Object.keys(studioFiles).length).toBeGreaterThanOrEqual(14);
    expect(studioSource).not.toContain("var-preview-strip");
    expect(studioSource).not.toContain("formatPreviewLine");
    expect(studioSource).toContain("<VarPreview text={p.content} />");
  });
});
