import { useEffect, useState, type CSSProperties } from "react";
import { springDurationMs, YAY } from "./motion";
import "./stackPanel.css";

export interface StackItem {
  id: number;
  /** Üst satır: ör. "bugün · 14:32" */
  meta: string;
  /** Tek satıra kırpılan önizleme */
  preview: string;
}

interface StackPanelProps {
  open: boolean;
  items: StackItem[];
  title: string;
  closeLabel: string;
  emptyText: string;
  actionLabel: string;
  onAction: (id: number) => void;
  onClose: () => void;
}

/** Kart yüksekliği + aralık (stackPanel.css ile aynı) */
const STEP_PX = 64;
const CARD_PX = 56;
/** Kapalıyken arkada görünen kart sayısı ve kaydırma */
const PEEK = 2;
const PEEK_OFFSET_PX = 7;
const PEEK_SCALE = 0.04;

/**
 * Yığın panel (R3.0d). Açılınca kartlar üst üste yığılı başlar ve yayla listeye dağılır;
 * kapanırken yeniden yığılır, sonra kaybolur. Yığılıyken kartlar etkileşime kapalıdır.
 */
export function StackPanel({
  open,
  items,
  title,
  closeLabel,
  emptyText,
  actionLabel,
  onAction,
  onClose,
}: StackPanelProps) {
  const [visible, setVisible] = useState(open);
  const [fanned, setFanned] = useState(false);

  useEffect(() => {
    if (open) {
      setVisible(true);
      // İki kare bekle: önce yığılı hâl çizilsin, sonra dağılma geçişi başlasın.
      let second = 0;
      const first = requestAnimationFrame(() => {
        second = requestAnimationFrame(() => setFanned(true));
      });
      return () => {
        cancelAnimationFrame(first);
        cancelAnimationFrame(second);
      };
    }
    setFanned(false);
    const id = window.setTimeout(() => setVisible(false), springDurationMs(YAY));
    return () => window.clearTimeout(id);
  }, [open]);

  if (!visible) return null;

  const height = fanned
    ? items.length * STEP_PX - (STEP_PX - CARD_PX)
    : CARD_PX + Math.min(items.length - 1, PEEK) * PEEK_OFFSET_PX;

  const cardStyle = (i: number): CSSProperties => {
    if (fanned) return { transform: `translateY(${i * STEP_PX}px)`, zIndex: items.length - i };
    const depth = Math.min(i, PEEK);
    return {
      transform: `translateY(${depth * PEEK_OFFSET_PX}px) scale(${1 - depth * PEEK_SCALE})`,
      opacity: i > PEEK ? 0 : 1 - depth * 0.25,
      zIndex: items.length - i,
    };
  };

  return (
    <section className={`meetin-stack ${fanned ? "is-fanned" : ""}`} aria-label={title}>
      <div className="meetin-stack-head">
        <span>{title}</span>
        <button type="button" className="meetin-stack-close" aria-label={closeLabel} title={closeLabel} onClick={onClose}>
          <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
            <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </div>
      {items.length === 0 ? (
        <p className="meetin-stack-empty">{emptyText}</p>
      ) : (
        <div className="meetin-stack-scroll">
          <ol className="meetin-stack-cards" style={{ height }}>
            {items.map((item, i) => (
              <li key={item.id} className="meetin-stack-card" style={cardStyle(i)} inert={!fanned}>
                <div className="meetin-stack-text">
                  <span className="meetin-stack-meta">{item.meta}</span>
                  <span className="meetin-stack-preview">{item.preview}</span>
                </div>
                <button type="button" className="btn btn-sm" onClick={() => onAction(item.id)}>
                  {actionLabel}
                </button>
              </li>
            ))}
          </ol>
        </div>
      )}
    </section>
  );
}
