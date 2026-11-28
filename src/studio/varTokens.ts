/**
 * Değişken hapları (R3.3, şartname §3.3): `{tarih:uzun}` gibi değişkenler düzenleyicide ve
 * listede ham yazı olarak değil, okunur bir hap olarak görünür. Kayıtta ham yazı korunur.
 * Sözdizimi Rust `vars::resolve_recursive` ile aynıdır; tanınmayan `{…}` düz yazı kalır.
 */
import { t } from "../i18n";
import { decodeHtmlEntities, escapeHtmlText } from "./richHtml";

export type VarSegment = { text: string } | { token: string; label: string };

const TOKEN_RE = /\{([^{}\n]+)\}/g;
const DATE_OFFSET_RE = /^[+-]\d+[gGhHaAyY]$/;

/** Ham değişkenin (`{…}` dahil) okunur adı; değişken değilse `null`. */
export function varLabel(token: string): string | null {
  if (!token.startsWith("{") || !token.endsWith("}")) return null;
  const inner = token.slice(1, -1);
  if (inner === "tarih") return t("var.date");
  if (inner === "tarih:uzun") return t("var.dateLong");
  if (inner.startsWith("tarih:")) {
    const spec = inner.slice("tarih:".length).trim();
    return DATE_OFFSET_RE.test(spec) ? `${t("var.date")} ${spec}` : null;
  }
  if (inner === "saat") return t("var.time");
  if (inner === "pano") return t("var.clipboard");
  if (inner === "imleç") return t("var.cursor");
  if (inner.startsWith("alan:")) {
    const name = inner.slice("alan:".length).split("|")[0].trim();
    return name ? `${t("var.field")}: ${name}` : null;
  }
  if (inner.startsWith("ifade:")) {
    const abbr = inner.slice("ifade:".length).trim();
    return abbr ? `${t("var.phrase")}: ${abbr}` : null;
  }
  return null;
}

/** Düz metni yazı ve değişken parçalarına ayırır. */
export function splitVarSegments(text: string): VarSegment[] {
  const out: VarSegment[] = [];
  let last = 0;
  for (const m of text.matchAll(TOKEN_RE)) {
    const label = varLabel(m[0]);
    if (label === null) continue;
    const start = m.index ?? 0;
    if (start > last) out.push({ text: text.slice(last, start) });
    out.push({ token: m[0], label });
    last = start + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
}

/** Tek satırlık düz önizleme (ipucu, yer kısıtlı alanlar): değişken yerine adı. */
export function previewText(content: string): string {
  return splitVarSegments(content)
    .map((s) => ("token" in s ? s.label : s.text))
    .join("")
    .replace(/\s+/g, " ")
    .trim();
}

function escapeAttr(raw: string): string {
  return escapeHtmlText(raw).replace(/"/g, "&quot;");
}

export function pillHtml(token: string, label: string): string {
  return `<span class="var-pill" contenteditable="false" data-var="${escapeAttr(token)}">${escapeHtmlText(label)}</span>`;
}

/**
 * Temizlenmiş HTML'deki değişkenleri haplara çevirir (yalnız düzenleyicide gösterim için).
 * Etiketlerin içine dokunmaz, yalnız metin parçalarını işler.
 */
export function pillifyHtml(html: string): string {
  if (!html) return "";
  return html.replace(
    /(<[^>]*>)|([^<]+)/g,
    (_all, tag: string | undefined, text: string | undefined) => {
      if (tag !== undefined) return tag;
      const decoded = decodeHtmlEntities(text ?? "");
      return splitVarSegments(decoded)
        .map((s) =>
          "token" in s ? pillHtml(s.token, s.label) : escapeHtmlText(s.text),
        )
        .join("");
    },
  );
}

const PILL_RE = /<span\b[^>]*\bdata-var="([^"]*)"[^>]*>[\s\S]*?<\/span>/gi;

/** Düzenleyici HTML'sindeki hapları ham değişken yazısına geri çevirir (kayıttan önce). */
export function unpillHtml(html: string): string {
  return html.replace(PILL_RE, (_all, attr: string) =>
    escapeHtmlText(decodeHtmlEntities(attr)),
  );
}

/** Yazarken imleçten önce tamamlanan değişken: `…{saat}` → `{saat}` (değişken değilse `null`). */
export function trailingVarToken(textBeforeCaret: string): string | null {
  const m = /\{[^{}\n]+\}$/.exec(textBeforeCaret);
  if (!m) return null;
  return varLabel(m[0]) === null ? null : m[0];
}
