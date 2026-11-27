import { useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { addTags } from "./tagLogic";
import "./tagInput.css";

interface TagInputProps {
  value: string[];
  onChange: (tags: string[]) => void;
  placeholder: string;
  /** Giriş kutusunun ekran okuyucu adı */
  inputLabel: string;
  /** Etiketi kaldırma düğmesinin ekran okuyucu adı ve ipucu */
  removeLabel: (tag: string) => string;
}

/** Çıkış animasyonu takılırsa (ör. CSS yüklenmedi) etiket yine de bu süre sonra kalkar. */
const LEAVE_FALLBACK_MS = 400;

/**
 * Etiket girişi (R3.0b). Enter, virgül ya da odaktan çıkınca yazılan etiket eklenir;
 * virgüllü yapıştırma birden çok etiket ekler. Boş kutuda Backspace önce son etiketi
 * işaretler, ikinci basışta siler. Etiketler ortak yayla açılır ve kapanır.
 */
export function TagInput({ value, onChange, placeholder, inputLabel, removeLabel }: TagInputProps) {
  const [text, setText] = useState("");
  const [marked, setMarked] = useState(false);
  const [leaving, setLeaving] = useState<string | null>(null);
  // Yalnız bu kutudan eklenen etiketler açılış animasyonu oynatır, düzenleyiciye gelenler değil.
  const fresh = useRef(new Set<string>());
  const inputRef = useRef<HTMLInputElement>(null);
  const leaveTimer = useRef<number | undefined>(undefined);
  // Çıkış animasyonu bitince en güncel listeyle çalışmak için
  const latest = useRef({ value, onChange });
  latest.current = { value, onChange };
  const leavingRef = useRef<string | null>(null);

  useEffect(() => () => window.clearTimeout(leaveTimer.current), []);

  const commit = (raw: string) => {
    const { tags, added } = addTags(value, raw);
    added.forEach((tag) => fresh.current.add(tag));
    if (added.length) onChange(tags);
    setMarked(false);
  };

  const finishLeave = (tag: string) => {
    if (leavingRef.current !== tag) return;
    window.clearTimeout(leaveTimer.current);
    leavingRef.current = null;
    setLeaving(null);
    fresh.current.delete(tag);
    latest.current.onChange(latest.current.value.filter((x) => x !== tag));
  };

  const remove = (tag: string) => {
    if (leavingRef.current) finishLeave(leavingRef.current);
    setMarked(false);
    leavingRef.current = tag;
    setLeaving(tag);
    leaveTimer.current = window.setTimeout(() => finishLeave(tag), LEAVE_FALLBACK_MS);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if ((e.key === "Enter" || e.key === ",") && text.trim()) {
      e.preventDefault();
      commit(text);
      setText("");
    } else if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
    } else if (e.key === "Backspace" && !text && value.length) {
      e.preventDefault();
      if (marked) remove(value[value.length - 1]);
      else setMarked(true);
    } else if (marked) {
      setMarked(false);
    }
  };

  const onPaste = (e: ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData("text");
    if (!pasted.includes(",")) return;
    e.preventDefault();
    commit(text + pasted);
    setText("");
  };

  const onBlur = () => {
    if (text.trim()) {
      commit(text);
      setText("");
    }
    setMarked(false);
  };

  return (
    <div className="meetin-tag-input" onClick={() => inputRef.current?.focus()}>
      {value.map((tag, i) => {
        const classes = ["meetin-tag"];
        if (fresh.current.has(tag)) classes.push("is-fresh");
        if (marked && i === value.length - 1) classes.push("is-marked");
        if (leaving === tag) classes.push("is-leaving");
        return (
          <span
            key={tag}
            className={classes.join(" ")}
            onAnimationEnd={(e) => {
              if (e.animationName === "meetinTagOut") finishLeave(tag);
            }}
          >
            #{tag}
            <button
              type="button"
              aria-label={removeLabel(tag)}
              title={removeLabel(tag)}
              onClick={(e) => {
                e.stopPropagation();
                remove(tag);
              }}
            >
              <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
                <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            </button>
          </span>
        );
      })}
      <input
        ref={inputRef}
        value={text}
        aria-label={inputLabel}
        placeholder={placeholder}
        onChange={(e) => {
          setText(e.target.value);
          setMarked(false);
        }}
        onKeyDown={onKeyDown}
        onPaste={onPaste}
        onBlur={onBlur}
      />
    </div>
  );
}
