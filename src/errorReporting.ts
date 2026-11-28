import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Yakalanmamış JS hatalarını Rust günlüğüne iletir (R1.4). Yalnız hata iletisi ve kaynak dosya
 * gönderilir; kullanıcı içeriği gönderilmez. Günlük yazımı başarısızsa sessizce geçilir. */
export function installErrorReporting(): void {
  let label = "?";
  try {
    label = getCurrentWindow().label;
  } catch {
    // Tauri dışında (test/tarayıcı) çalışıyor.
  }
  const send = (kind: string, message: string, source?: string) => {
    invoke("log_frontend_error", { window: label, kind, message, source }).catch(() => {});
  };
  window.addEventListener("error", (e) => {
    send("error", e.message || String(e.error), e.filename ? `${e.filename}:${e.lineno}` : undefined);
  });
  // CSP ihlali: hangi yönerge hangi kaynak türünü engelledi (adres yazılmaz, R1.7)
  document.addEventListener("securitypolicyviolation", (e) => {
    const kind = e.blockedURI.split(":")[0] || "inline";
    send("csp", `${e.effectiveDirective} engelledi: ${kind}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const r: unknown = e.reason;
    send("unhandledrejection", r instanceof Error ? r.message : String(r));
  });
}

/**
 * Yakalanan ama kullanıcıya gösterilecek yeri olmayan hatayı günlüğe yazar (maskot, ada).
 * Yalnız bağlam adı ve hata iletisi gider; ifade içeriği ya da tuş vuruşu gönderilmez.
 */
export function reportCaughtError(context: string, err: unknown): void {
  let label = "?";
  try {
    label = getCurrentWindow().label;
  } catch {
    // Tauri dışında (test/tarayıcı) çalışıyor.
  }
  const message = err instanceof Error ? err.message : String(err);
  invoke("log_frontend_error", { window: label, kind: context, message }).catch(() => {
    // Günlük yazılamıyorsa yapılacak başka bir şey yok (sonsuz döngüye girilmez).
  });
}
