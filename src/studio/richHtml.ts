import { escapeHtmlAttr, isSafeEmbeddedImageDataUrl } from "./richImages";

const DROP_WITH_CONTENT_TAGS = new Set([
  "script",
  "style",
  "iframe",
  "object",
  "embed",
  "noscript",
  "template",
  "svg",
  "math",
  "form",
  "input",
  "textarea",
  "button",
  "select",
  "meta",
  "link",
  "base",
  "head",
  "title",
]);

const VOID_TAGS = new Set(["br", "img", "hr", "wbr", "input", "meta", "link", "base"]);

const INLINE_FORMAT_TAGS = new Set(["b", "strong", "i", "em", "u", "a"]);
const RICH_MARKER_TAG_RE = /<(?:b|strong|i|em|u|a|ul|ol|li|img)\b/i;

export function escapeHtmlText(raw: string): string {
  return raw
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function decodeHtmlEntities(raw: string): string {
  return raw.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (full, body: string) => {
    const low = body.toLowerCase();
    if (low === "amp") return "&";
    if (low === "lt") return "<";
    if (low === "gt") return ">";
    if (low === "quot") return '"';
    if (low === "apos") return "'";
    if (low === "nbsp") return " ";
    if (low.startsWith("#x")) {
      const code = Number.parseInt(low.slice(2), 16);
      if (Number.isFinite(code) && code > 0 && code <= 0x10ffff) {
        return String.fromCodePoint(code);
      }
      return full;
    }
    if (low.startsWith("#")) {
      const code = Number.parseInt(low.slice(1), 10);
      if (Number.isFinite(code) && code > 0 && code <= 0x10ffff) {
        return String.fromCodePoint(code);
      }
      return full;
    }
    return full;
  });
}

/**
 * Yalnızca `http://`, `https://` ve `mailto:` protokollerine izin verir.
 * HTML varlıklarını çözer ve kontrol karakterleriyle gizlenmiş `javascript:` vb. reddeder.
 */
export function isSafeLinkUrl(raw: string): boolean {
  const decoded = decodeHtmlEntities(raw)
    .replace(/[\u0000-\u001f\u007f]+/g, "")
    .trim();
  if (!decoded) return false;
  return /^(?:https?:\/\/[^\s]+|mailto:[^\s@]+@[^\s@]+)$/i.test(decoded);
}

export function normalizeSafeLinkUrl(raw: string): string | null {
  const decoded = decodeHtmlEntities(raw)
    .replace(/[\u0000-\u001f\u007f]+/g, "")
    .trim();
  if (!isSafeLinkUrl(decoded)) return null;
  return decoded;
}

interface ParsedAttrMap {
  [name: string]: string;
}

function parseTagAttributes(attrSource: string): ParsedAttrMap {
  const attrs: ParsedAttrMap = {};
  const attrRe = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*(?:=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
  let match: RegExpExecArray | null;
  while ((match = attrRe.exec(attrSource)) !== null) {
    const name = match[1].toLowerCase();
    const rawVal = match[2] ?? match[3] ?? match[4] ?? "";
    attrs[name] = decodeHtmlEntities(rawVal);
  }
  return attrs;
}

interface StyleFlags {
  bold: boolean;
  italic: boolean;
  underline: boolean;
}

function extractInlineStyleFlags(style: string | undefined): StyleFlags {
  if (!style) {
    return { bold: false, italic: false, underline: false };
  }
  const low = style.toLowerCase();
  const bold =
    /font-weight\s*:\s*(?:bold|bolder|[6-9]00)\b/.test(low);
  const italic = /font-style\s*:\s*(?:italic|oblique)\b/.test(low);
  const underline = /text-decoration(?:-[a-z]+)?\s*:[^;]*\bunderline\b/.test(low);
  return { bold, italic, underline };
}

interface OpenFrame {
  sourceTag: string;
  emitTags: string[];
}

/**
 * Zengin metin HTML'sini beyaz liste tabanlı temizleyiciden geçirir.
 * İzin verilen etiketler: `b`, `i`, `u`, `a` (yalnızca http/https/mailto), `ul`, `ol`, `li`, `p`, `br`, `img` (yalnızca güvenli data:image base64).
 * `<script>`, `<style>`, `<iframe>`, `on*` olay işleyicileri ve `javascript:` URL'leri tamamen temizlenir.
 */
export function sanitizeRichHtml(rawHtml: string): string {
  if (!rawHtml) return "";

  // Yorumları (Word conditional comments dahil) önce temizle
  const withoutComments = rawHtml.replace(/<!--[\s\S]*?-->/g, "");

  const tokenRe = /<(\/?)([a-zA-Z][a-zA-Z0-9:-]*)([^<>]*?)(\/?)>|([^<]+)/g;
  let match: RegExpExecArray | null;
  const out: string[] = [];
  const stack: OpenFrame[] = [];
  const droppedStack: string[] = [];

  const pushCloseTags = (tags: string[]) => {
    for (let i = tags.length - 1; i >= 0; i -= 1) {
      out.push(`</${tags[i]}>`);
    }
  };

  while ((match = tokenRe.exec(withoutComments)) !== null) {
    const [, slash, rawTagName, attrSource, selfSlash, textChunk] = match;

    if (textChunk !== undefined) {
      if (droppedStack.length === 0) {
        out.push(escapeHtmlText(decodeHtmlEntities(textChunk)));
      }
      continue;
    }

    const tag = rawTagName.toLowerCase();
    const isClose = slash === "/";
    const isSelfClose = selfSlash === "/" || VOID_TAGS.has(tag);

    if (droppedStack.length > 0) {
      if (isClose) {
        const idx = droppedStack.lastIndexOf(tag);
        if (idx !== -1) {
          droppedStack.splice(idx);
        }
      } else if (!isSelfClose && DROP_WITH_CONTENT_TAGS.has(tag)) {
        droppedStack.push(tag);
      }
      continue;
    }

    if (!isClose && DROP_WITH_CONTENT_TAGS.has(tag)) {
      if (!isSelfClose) {
        droppedStack.push(tag);
      }
      continue;
    }

    if (isClose) {
      let idx = -1;
      for (let i = stack.length - 1; i >= 0; i -= 1) {
        if (stack[i].sourceTag === tag) {
          idx = i;
          break;
        }
      }
      if (idx !== -1) {
        while (stack.length > idx) {
          const frame = stack.pop()!;
          pushCloseTags(frame.emitTags);
        }
      }
      continue;
    }

    // Açılış etiketi
    if (tag === "br") {
      out.push("<br>");
      continue;
    }

    if (tag === "img") {
      const attrs = parseTagAttributes(attrSource);
      const src = (attrs.src ?? "").trim();
      if (isSafeEmbeddedImageDataUrl(src)) {
        const alt = (attrs.alt ?? "").trim();
        if (alt) {
          out.push(`<img src="${escapeHtmlAttr(src)}" alt="${escapeHtmlAttr(alt)}" />`);
        } else {
          out.push(`<img src="${escapeHtmlAttr(src)}" />`);
        }
      }
      continue;
    }

    if (isSelfClose) {
      continue;
    }

    if (tag === "b" || tag === "strong") {
      out.push("<b>");
      stack.push({ sourceTag: tag, emitTags: ["b"] });
      continue;
    }

    if (tag === "i" || tag === "em") {
      out.push("<i>");
      stack.push({ sourceTag: tag, emitTags: ["i"] });
      continue;
    }

    if (tag === "u") {
      out.push("<u>");
      stack.push({ sourceTag: tag, emitTags: ["u"] });
      continue;
    }

    if (tag === "a") {
      const attrs = parseTagAttributes(attrSource);
      const safeHref = normalizeSafeLinkUrl(attrs.href ?? "");
      if (safeHref) {
        out.push(`<a href="${escapeHtmlAttr(safeHref)}">`);
        stack.push({ sourceTag: tag, emitTags: ["a"] });
      } else {
        stack.push({ sourceTag: tag, emitTags: [] });
      }
      continue;
    }

    if (tag === "ul" || tag === "ol" || tag === "li") {
      out.push(`<${tag}>`);
      stack.push({ sourceTag: tag, emitTags: [tag] });
      continue;
    }

    if (tag === "p" || tag === "div") {
      out.push("<p>");
      stack.push({ sourceTag: tag, emitTags: ["p"] });
      continue;
    }

    if (/^h[1-6]$/.test(tag)) {
      out.push("<p><b>");
      stack.push({ sourceTag: tag, emitTags: ["p", "b"] });
      continue;
    }

    if (tag === "span" || tag === "font") {
      const attrs = parseTagAttributes(attrSource);
      const flags = extractInlineStyleFlags(attrs.style);
      const emitTags: string[] = [];
      if (flags.bold) {
        out.push("<b>");
        emitTags.push("b");
      }
      if (flags.italic) {
        out.push("<i>");
        emitTags.push("i");
      }
      if (flags.underline) {
        out.push("<u>");
        emitTags.push("u");
      }
      stack.push({ sourceTag: tag, emitTags });
      continue;
    }

    // Diğer güvenli kapsayıcı etiketler (ör. section, article, tbody): etiketi soy, iç metni koru
    stack.push({ sourceTag: tag, emitTags: [] });
  }

  while (stack.length > 0) {
    const frame = stack.pop()!;
    pushCloseTags(frame.emitTags);
  }

  return cleanupSanitizedHtml(out.join(""));
}

function cleanupSanitizedHtml(html: string): string {
  let res = html;
  // Boş biçim etiketlerini temizle (<img> içerenleri koru)
  let prev = "";
  while (res !== prev) {
    prev = res;
    res = res.replace(/<(b|i|u|a)(?:\s+[^>]*)?><\/\1>/gi, "");
    res = res.replace(/<(ul|ol)><\/\1>/gi, "");
    res = res.replace(/<p><br><\/p>/gi, "<p></p>");
  }
  // Tek bir <p>...</p> sarmalayıcısını sadeleştir (liste/çoklu paragraf değilse)
  res = res.replace(/^(?:<p><\/p>)+/gi, "").replace(/(?:<p><\/p>)+$/gi, "");
  res = res.replace(/(?:<br>)+$/gi, "");
  return res.trim();
}

export function plainTextToRichHtml(text: string): string {
  if (!text) return "";
  return escapeHtmlText(text).replace(/\r?\n/g, "<br>");
}

export function hasRichFormatting(html: string | null | undefined): boolean {
  if (!html) return false;
  return RICH_MARKER_TAG_RE.test(html);
}

/**
 * Temiz HTML'den düz metin çıkarır. Madde işaretli ve numaralı listeleri satır bazlı korur.
 */
export function richHtmlToPlainText(html: string): string {
  if (!html) return "";
  const sanitized = sanitizeRichHtml(html);
  const tokenRe = /<(\/?)([a-zA-Z][a-zA-Z0-9:-]*)([^<>]*?)(\/?)>|([^<]+)/g;
  let match: RegExpExecArray | null;
  let out = "";
  const listStack: Array<{ type: "ul" | "ol"; index: number }> = [];

  const ensureNewline = () => {
    if (out.length > 0 && !out.endsWith("\n")) {
      out += "\n";
    }
  };

  while ((match = tokenRe.exec(sanitized)) !== null) {
    const [, slash, rawTagName, , , textChunk] = match;
    if (textChunk !== undefined) {
      out += decodeHtmlEntities(textChunk).replace(/\u00a0/g, " ");
      continue;
    }

    const tag = rawTagName.toLowerCase();
    const isClose = slash === "/";

    if (tag === "br") {
      out += "\n";
    } else if (tag === "ul") {
      if (!isClose) {
        ensureNewline();
        listStack.push({ type: "ul", index: 0 });
      } else {
        listStack.pop();
        ensureNewline();
      }
    } else if (tag === "ol") {
      if (!isClose) {
        ensureNewline();
        listStack.push({ type: "ol", index: 0 });
      } else {
        listStack.pop();
        ensureNewline();
      }
    } else if (tag === "li") {
      if (!isClose) {
        ensureNewline();
        const top = listStack[listStack.length - 1];
        if (top && top.type === "ol") {
          top.index += 1;
          out += `${top.index}. `;
        } else {
          out += "• ";
        }
      } else {
        ensureNewline();
      }
    } else if (tag === "p" || tag === "div") {
      if (isClose) {
        ensureNewline();
      } else if (out.length > 0 && !out.endsWith("\n")) {
        out += "\n";
      }
    }
  }

  return out.replace(/\r/g, "").replace(/\n+$/, "");
}

export interface SyncedRichContent {
  content: string;
  content_html: string | null;
}

/**
 * Canlı düzenleyici HTML'sini temizler ve düz metin (`content`) ↔ zengin HTML (`content_html`) ikilisini senkronize eder.
 * Zengin biçimlendirme yoksa `content_html` değerini `null` döndürür.
 */
export function syncEditorHtml(rawHtml: string): SyncedRichContent {
  const cleanHtml = sanitizeRichHtml(rawHtml);
  const plain = richHtmlToPlainText(cleanHtml);
  if (!hasRichFormatting(cleanHtml)) {
    return {
      content: plain,
      content_html: null,
    };
  }
  return {
    content: plain,
    content_html: cleanHtml,
  };
}

/**
 * Düzenleyici ilk yüklenirken `content` ve `content_html` değerlerinden güvenli başlangıç HTML'si üretir.
 */
export function toEditorInitialHtml(
  content: string,
  contentHtml: string | null,
): string {
  if (contentHtml && contentHtml.trim().length > 0) {
    return sanitizeRichHtml(contentHtml);
  }
  return plainTextToRichHtml(content);
}

/**
 * İzinli satır içi biçim etiketi olup olmadığını döndürür.
 */
export function isAllowedInlineTag(tag: string): boolean {
  return INLINE_FORMAT_TAGS.has(tag.toLowerCase());
}
