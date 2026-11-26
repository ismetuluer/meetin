import Fuse from "fuse.js";
import type { PhraseRecord } from "../studio/types";

export interface PhraseSearchIndex {
  search: (query: string, limit?: number) => PhraseRecord[];
}

interface IndexedPhrase {
  phrase: PhraseRecord;
  abbrLower: string;
  descLower: string;
  contentLower: string;
  tagsLower: string;
}

/**
 * Türkçe duyarlı küçük harfe çevirme.
 */
function trLower(text: string): string {
  return text.toLocaleLowerCase("tr-TR");
}

/**
 * 10.000 ifadede < 30 ms hedefli bulanık arama indeksi (§5.2, §10.1, M2.4).
 * Önceden normalize edilmiş alanlarla alt-dize taraması milisaniyenin altında çalışır;
 * yazım hatası toleransı için `fuse.js` yedeği devrededir.
 */
export function createPhraseSearchIndex(phrases: PhraseRecord[]): PhraseSearchIndex {
  const active = phrases
    .filter((p) => p.enabled && p.deleted_at === null)
    .sort((a, b) => b.use_count - a.use_count || b.id - a.id);

  const prepared: IndexedPhrase[] = active.map((phrase) => ({
    phrase,
    abbrLower: trLower(phrase.abbr),
    descLower: trLower(phrase.description),
    contentLower: trLower(phrase.content),
    tagsLower: phrase.tags.map(trLower).join(" "),
  }));

  const fuse = new Fuse(active, {
    keys: [
      { name: "abbr", weight: 0.45 },
      { name: "description", weight: 0.25 },
      { name: "content", weight: 0.2 },
      { name: "tags", weight: 0.1 },
    ],
    threshold: 0.35,
    ignoreLocation: true,
    minMatchCharLength: 1,
  });

  return {
    search(query: string, limit = 30): PhraseRecord[] {
      const q = query.trim();
      if (!q) {
        return active.slice(0, limit);
      }

      const qLower = trLower(q);
      const exactOrPrefix: PhraseRecord[] = [];
      const otherMatches: PhraseRecord[] = [];

      for (let i = 0; i < prepared.length; i++) {
        const item = prepared[i];
        if (item.abbrLower.startsWith(qLower)) {
          exactOrPrefix.push(item.phrase);
          if (exactOrPrefix.length >= limit) {
            return exactOrPrefix;
          }
        } else if (
          otherMatches.length < limit &&
          (item.abbrLower.includes(qLower) ||
            item.descLower.includes(qLower) ||
            item.contentLower.includes(qLower) ||
            item.tagsLower.includes(qLower))
        ) {
          otherMatches.push(item.phrase);
        }
      }

      if (exactOrPrefix.length > 0 || otherMatches.length > 0) {
        return exactOrPrefix.concat(otherMatches).slice(0, limit);
      }

      return fuse.search(q, { limit }).map((r) => r.item);
    },
  };
}
