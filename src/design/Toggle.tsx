import type { InputHTMLAttributes } from "react";
import "./toggle.css";

type ToggleProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "role">;

/**
 * Aç/kapa anahtarı (R3.0a). Yerel onay kutusudur: Boşluk tuşu, etiket tıklaması ve
 * pasif durum tarayıcıdan gelir; ekran okuyucu `role="switch"` ile anahtar olarak okur.
 */
export function Toggle({ className, ...rest }: ToggleProps) {
  return (
    <input
      {...rest}
      type="checkbox"
      role="switch"
      className={className ? `meetin-toggle ${className}` : "meetin-toggle"}
    />
  );
}
