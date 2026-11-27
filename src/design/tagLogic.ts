/* Etiket girişinin saf mantığı (R3.0b). Karşılaştırmalar tr-TR kurallarıyla yapılır. */

/** Baştaki `#` işaretlerini ve kenar boşluklarını atar. */
export function normalizeTag(raw: string): string {
  return raw.trim().replace(/^#+/, "").trim();
}

/** Virgülle ayrılmış metni etiketlere böler; boş parçalar atlanır. */
export function parseTags(raw: string): string[] {
  return raw.split(",").map(normalizeTag).filter(Boolean);
}

const key = (tag: string) => tag.toLocaleLowerCase("tr-TR");

/**
 * Metindeki etiketleri ekler. Var olan (büyük/küçük harf farkıyla bile) etiket tekrar eklenmez.
 * Hiçbir şey eklenmezse aynı dizi döner.
 */
export function addTags(existing: string[], raw: string): { tags: string[]; added: string[] } {
  const seen = new Set(existing.map(key));
  const added: string[] = [];
  for (const tag of parseTags(raw)) {
    if (seen.has(key(tag))) continue;
    seen.add(key(tag));
    added.push(tag);
  }
  return { tags: added.length ? [...existing, ...added] : existing, added };
}
