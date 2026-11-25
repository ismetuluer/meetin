import tr from "./tr.json";

export type MetinAnahtari = keyof typeof tr;

const sozluk: Record<string, string> = tr;

/** Arayüz metnini döndürür. Eksik anahtar geliştirme sırasında hata fırlatır. */
export function t(anahtar: MetinAnahtari): string {
  const metin = sozluk[anahtar];
  if (metin === undefined) throw new Error(`Eksik metin anahtarı: ${anahtar}`);
  return metin;
}
