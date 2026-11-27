import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import css from "./tagInput.css?raw";
import { TagInput } from "./TagInput";

describe("TagInput (R3.0b)", () => {
  const html = renderToStaticMarkup(
    <TagInput
      value={["resmi", "İş"]}
      onChange={() => {}}
      placeholder="+ etiket"
      inputLabel="Etiketler"
      removeLabel={(tag) => `Etiketi kaldır: ${tag}`}
    />,
  );

  it("her etiketi ve adlandırılmış kaldırma düğmesini gösterir", () => {
    expect(html).toContain("#resmi");
    expect(html).toContain("#İş");
    expect(html).toContain('aria-label="Etiketi kaldır: resmi"');
    expect(html).toContain('aria-label="Etiketi kaldır: İş"');
  });

  it("giriş kutusunun ekran okuyucu adı var", () => {
    expect(html).toContain('aria-label="Etiketler"');
    expect(html).toContain('placeholder="+ etiket"');
  });

  it("düzenleyiciye gelen etiketler açılış animasyonu oynatmaz", () => {
    expect(html).not.toContain("is-fresh");
  });

  it("ortak yayı ve yalnız §11.4 renk rollerini kullanır", () => {
    expect(css).toContain("var(--yay-sure-kisa) var(--yay-egri)");
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(/i);
  });
});
