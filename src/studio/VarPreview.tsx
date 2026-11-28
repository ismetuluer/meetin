import { splitVarSegments } from "./varTokens";

/** Tek satırlık önizleme: değişkenler küçük hap, yazı boşlukları sadeleşmiş (R3.3). */
export function VarPreview({ text }: { text: string }) {
  const segments = splitVarSegments(text.replace(/\s+/g, " ").trim());
  return (
    <>
      {segments.map((s, i) =>
        "token" in s ? (
          <span key={i} className="var-pill is-mini" title={s.token}>
            {s.label}
          </span>
        ) : (
          s.text
        ),
      )}
    </>
  );
}
