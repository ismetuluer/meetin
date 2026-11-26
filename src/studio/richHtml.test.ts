import { describe, expect, it } from "vitest";
import {
  hasRichFormatting,
  isSafeLinkUrl,
  normalizeSafeLinkUrl,
  plainTextToRichHtml,
  richHtmlToPlainText,
  sanitizeRichHtml,
  syncEditorHtml,
  toEditorInitialHtml,
} from "./richHtml";

const SAMPLE_PNG_DATA_URL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

describe("richHtml sanitizer & serializer", () => {
  it("preserves bold, italic, underline, links, and lists cleanly", () => {
    const input =
      '<p><strong>Merhaba</strong> <em>dünya</em> <u>altı çizili</u></p>' +
      '<ul><li>Madde bir</li><li>Madde iki</li></ul>' +
      '<ol><li>İlk adım</li><li>İkinci adım</li></ol>' +
      '<a href="https://example.com/docs" target="_blank" onclick="evil()">Belge</a>';

    const clean = sanitizeRichHtml(input);
    expect(clean).toBe(
      '<p><b>Merhaba</b> <i>dünya</i> <u>altı çizili</u></p>' +
        '<ul><li>Madde bir</li><li>Madde iki</li></ul>' +
        '<ol><li>İlk adım</li><li>İkinci adım</li></ol>' +
        '<a href="https://example.com/docs">Belge</a>',
    );
    expect(hasRichFormatting(clean)).toBe(true);
  });

  it("extracts readable plain text from paragraphs, breaks, links, and lists", () => {
    const html =
      "<p><b>Sayın</b> {alan:İsim},</p>" +
      "<ul><li>Öğe 1</li><li>Öğe 2</li></ul>" +
      "<ol><li>Birinci</li><li>İkinci</li></ol>" +
      '<p><a href="mailto:destek@ornek.com">Destek &amp; Bilgi</a><br>İyi günler</p>';

    expect(richHtmlToPlainText(html)).toBe(
      "Sayın {alan:İsim},\n• Öğe 1\n• Öğe 2\n1. Birinci\n2. İkinci\nDestek & Bilgi\nİyi günler",
    );
  });

  it("converts pasted inline bold/italic/underline spans into semantic tags", () => {
    const pasted =
      '<span style="font-weight: 700; color: red;">Kalın</span> ' +
      '<span style="font-style: italic; text-decoration: underline;">Eğik Altı Çizili</span>';

    expect(sanitizeRichHtml(pasted)).toBe(
      "<b>Kalın</b> <i><u>Eğik Altı Çizili</u></i>",
    );
  });

  it("rejects XSS vectors: script, style, iframe, svg, event handlers, and javascript URLs", () => {
    const xss = [
      '<script>alert("xss")</script>',
      "<style>body { display: none; }</style>",
      '<iframe src="https://evil.example"></iframe>',
      '<svg onload="alert(1)"><circle /></svg>',
      '<b onclick="alert(1)" onmouseover="steal()">Güvenli Metin</b>',
      '<a href="javascript:alert(1)">Kötü Link</a>',
      '<a href="&#106;avascript:alert(2)">Gizli Link</a>',
      '<a href="java\tscript:alert(3)">Sekmeli Link</a>',
      '<a href="data:text/html,<script>alert(1)</script>">Veri Linki</a>',
      '<img src="javascript:alert(1)" onerror="alert(2)" alt="x" />',
      '<img src="https://tracker.example/pixel.png" alt="remote" />',
    ].join("");

    const clean = sanitizeRichHtml(xss);
    expect(clean).not.toContain("<script");
    expect(clean).not.toContain("alert(");
    expect(clean).not.toContain("<style");
    expect(clean).not.toContain("<iframe");
    expect(clean).not.toContain("<svg");
    expect(clean).not.toContain("onclick");
    expect(clean).not.toContain("onerror");
    expect(clean).not.toContain("onmouseover");
    expect(clean).not.toContain("javascript:");
    expect(clean).not.toContain("tracker.example");
    expect(clean).toContain("<b>Güvenli Metin</b>");
    expect(clean).toContain("Kötü Link");
    expect(clean).toContain("Gizli Link");
  });

  it("allows only safe data:image base64 images", () => {
    const html = `<p>Logo:<br><img src="${SAMPLE_PNG_DATA_URL}" alt="Örnek Logo" onerror="alert(1)" /></p>`;
    const clean = sanitizeRichHtml(html);
    expect(clean).toBe(
      `<p>Logo:<br><img src="${SAMPLE_PNG_DATA_URL}" alt="Örnek Logo" /></p>`,
    );
    expect(hasRichFormatting(clean)).toBe(true);
  });

  it("validates allowed link protocols strictly", () => {
    expect(isSafeLinkUrl("https://example.com")).toBe(true);
    expect(isSafeLinkUrl("http://example.com/a?b=1")).toBe(true);
    expect(isSafeLinkUrl("mailto:destek@ornek.com")).toBe(true);
    expect(normalizeSafeLinkUrl("  https://example.com  ")).toBe("https://example.com");

    expect(isSafeLinkUrl("javascript:alert(1)")).toBe(false);
    expect(isSafeLinkUrl("vbscript:msgbox(1)")).toBe(false);
    expect(isSafeLinkUrl("data:text/html,hi")).toBe(false);
    expect(isSafeLinkUrl("file:///C:/Windows/win.ini")).toBe(false);
    expect(isSafeLinkUrl("example.com")).toBe(false);
    expect(isSafeLinkUrl("")).toBe(false);
  });

  it("keeps content_html null when editor contains only plain text and preserves HTML when formatted", () => {
    const plainSync = syncEditorHtml("<div>Satır 1</div><div>Satır 2</div>");
    expect(plainSync).toEqual({
      content: "Satır 1\nSatır 2",
      content_html: null,
    });

    const richSync = syncEditorHtml("<p>Merhaba <b>Meetin</b>!</p>");
    expect(richSync).toEqual({
      content: "Merhaba Meetin!",
      content_html: "<p>Merhaba <b>Meetin</b>!</p>",
    });
  });

  it("builds safe initial editor HTML from plain text or stored content_html", () => {
    expect(plainTextToRichHtml("A & B\nC < D")).toBe("A &amp; B<br>C &lt; D");
    expect(toEditorInitialHtml("Merhaba\nDünya", null)).toBe("Merhaba<br>Dünya");
    expect(
      toEditorInitialHtml("Merhaba", '<b>Merhaba</b><script>alert(1)</script>'),
    ).toBe("<b>Merhaba</b>");
  });
});
