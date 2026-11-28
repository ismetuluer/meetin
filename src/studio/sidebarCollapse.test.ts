import { describe, expect, it } from "vitest";
import css from "./studio.css?raw";

// R4.2: StudioView.tsx bileşenlere bölündü; denetim bütün stüdyo kabuğu kaynağında yapılır.
const studioFiles = import.meta.glob<string>(
  ["./Studio*.tsx", "./studio*.ts", "./useStudio*.ts", "!./*.test.*"],
  { query: "?raw", import: "default", eager: true },
);
const studioSource = Object.values(studioFiles).join("\n");

describe("kenar çubuğu daraltma (R3.0c)", () => {
  it("yazı ve sayılar DOM'dan silinmez, yalnız solar (ekran okuyucu adı korunur)", () => {
    expect(Object.keys(studioFiles).length).toBeGreaterThanOrEqual(14);
    expect(studioSource).toContain('className="nav-text"');
    expect(studioSource).not.toMatch(/!sidebarCollapsed && <span>/);
    expect(studioSource).not.toMatch(/!sidebarCollapsed && \(?\s*<span className="nav-count">/);
  });

  it("daraltma düğmesi adlandırılmış ve durumunu bildiriyor", () => {
    expect(studioSource).toContain("aria-expanded={!sidebarCollapsed}");
    expect(studioSource).toMatch(/aria-label=\{sidebarCollapsed \? t\("studio\.expandSidebar"\)/);
  });

  it("genişlik ve ok ortak yayla hareket eder", () => {
    const text = css.replace(/\r\n/g, "\n");
    expect(text).toContain("transition: grid-template-columns var(--yay-sure-kisa) var(--yay-egri);");
    expect(text).toContain("transition: transform var(--yay-sure-kisa) var(--yay-egri);");
    expect(text).toMatch(/\.fluent-sidebar\.is-collapsed \.nav-text,\n\.fluent-sidebar\.is-collapsed \.nav-count \{\n  opacity: 0;/);
  });
});
