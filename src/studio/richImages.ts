export const MAX_EMBEDDED_IMAGE_BYTES = 2 * 1024 * 1024;

const ALLOWED_MIME_TYPES = new Set([
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/svg+xml",
]);

const ALLOWED_EXTENSIONS = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg"];

const SAFE_DATA_IMAGE_URI_RE =
  /^data:image\/(?:png|jpeg|gif|webp|svg\+xml);base64,[A-Za-z0-9+/=\s]+$/i;

export interface EmbeddedImagePreview {
  key: string;
  src: string;
  alt: string;
  matchStart: number;
  matchEnd: number;
}

export function isSupportedImageFile(file: { type?: string; name?: string }): boolean {
  const mime = (file.type ?? "").trim().toLowerCase();
  if (mime && ALLOWED_MIME_TYPES.has(mime)) {
    return true;
  }
  const name = (file.name ?? "").trim().toLowerCase();
  return ALLOWED_EXTENSIONS.some((ext) => name.endsWith(ext));
}

export function escapeHtmlAttr(raw: string): string {
  return raw
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function unescapeHtmlAttr(raw: string): string {
  return raw
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

export function buildEmbeddedImageTag(dataUrl: string, alt: string): string {
  const safeAlt = escapeHtmlAttr(alt.trim() || "Görsel");
  return `<img src="${dataUrl}" alt="${safeAlt}" />`;
}

export function isSafeEmbeddedImageDataUrl(uri: string): boolean {
  return SAFE_DATA_IMAGE_URI_RE.test(uri.trim());
}

/**
 * `content_html` içindeki güvenli `data:image/...;base64,...` `<img>` öğelerini
 * `dangerouslySetInnerHTML` kullanmadan önizlemek üzere ayrıştırır.
 */
export function extractEmbeddedImages(html: string | null): EmbeddedImagePreview[] {
  if (!html) return [];
  const results: EmbeddedImagePreview[] = [];
  const imgTagRe = /<img\b[^>]*>/gi;
  let match: RegExpExecArray | null;
  let idx = 0;

  while ((match = imgTagRe.exec(html)) !== null) {
    const tag = match[0];
    const srcMatch = /\bsrc\s*=\s*(?:"([^"]*)"|'([^']*)')/i.exec(tag);
    const src = (srcMatch?.[1] ?? srcMatch?.[2] ?? "").trim();
    if (!src || !isSafeEmbeddedImageDataUrl(src)) {
      continue;
    }
    const altMatch = /\balt\s*=\s*(?:"([^"]*)"|'([^']*)')/i.exec(tag);
    const rawAlt = altMatch?.[1] ?? altMatch?.[2] ?? "";
    results.push({
      key: `${idx}-${match.index}`,
      src,
      alt: unescapeHtmlAttr(rawAlt).trim(),
      matchStart: match.index,
      matchEnd: match.index + tag.length,
    });
    idx += 1;
  }

  return results;
}

export function removeEmbeddedImageAt(
  html: string,
  matchStart: number,
  matchEnd: number,
): string | null {
  let before = html.slice(0, matchStart);
  let after = html.slice(matchEnd);

  if (/<br\s*\/?>\s*$/i.test(before)) {
    before = before.replace(/<br\s*\/?>\s*$/i, "");
  } else if (/^\s*<br\s*\/?>/i.test(after)) {
    after = after.replace(/^\s*<br\s*\/?>/i, "");
  }

  const next = `${before}${after}`.trim();
  return next.length > 0 ? next : null;
}
