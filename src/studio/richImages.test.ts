import { describe, expect, it } from "vitest";
import {
  buildEmbeddedImageTag,
  extractEmbeddedImages,
  isSafeEmbeddedImageDataUrl,
  isSupportedImageFile,
  removeEmbeddedImageAt,
} from "./richImages";

const SAMPLE_PNG_DATA_URL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

describe("richImages helpers", () => {
  it("validates supported image MIME types and extensions", () => {
    expect(isSupportedImageFile({ type: "image/png", name: "a.png" })).toBe(true);
    expect(isSupportedImageFile({ type: "image/jpeg", name: "a.jpg" })).toBe(true);
    expect(isSupportedImageFile({ type: "image/gif", name: "a.gif" })).toBe(true);
    expect(isSupportedImageFile({ type: "image/webp", name: "a.webp" })).toBe(true);
    expect(isSupportedImageFile({ type: "image/svg+xml", name: "a.svg" })).toBe(true);
    expect(isSupportedImageFile({ type: "", name: "logo.PNG" })).toBe(true);
    expect(isSupportedImageFile({ type: "application/pdf", name: "doc.pdf" })).toBe(false);
    expect(isSupportedImageFile({ type: "text/html", name: "x.html" })).toBe(false);
  });

  it("extracts only safe data:image base64 previews and ignores external/unsafe sources", () => {
    const html = [
      "<b>Merhaba</b><br>",
      buildEmbeddedImageTag(SAMPLE_PNG_DATA_URL, 'logo "1"'),
      '<img src="https://example.com/tracker.png" alt="ext" />',
      '<img src="javascript:alert(1)" alt="bad" />',
    ].join("");

    const images = extractEmbeddedImages(html);
    expect(images).toHaveLength(1);
    expect(images[0].src).toBe(SAMPLE_PNG_DATA_URL);
    expect(images[0].alt).toBe('logo "1"');
    expect(isSafeEmbeddedImageDataUrl("javascript:alert(1)")).toBe(false);
  });

  it("removes embedded image cleanly along with separator break", () => {
    const html = `<b>Merhaba</b><br>${buildEmbeddedImageTag(SAMPLE_PNG_DATA_URL, "logo")}`;
    const [img] = extractEmbeddedImages(html);
    expect(removeEmbeddedImageAt(html, img.matchStart, img.matchEnd)).toBe("<b>Merhaba</b>");

    const onlyImg = buildEmbeddedImageTag(SAMPLE_PNG_DATA_URL, "logo");
    const [single] = extractEmbeddedImages(onlyImg);
    expect(removeEmbeddedImageAt(onlyImg, single.matchStart, single.matchEnd)).toBeNull();
  });
});
