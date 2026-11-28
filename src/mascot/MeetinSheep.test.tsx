import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import css from "./mascot.css?raw";
import { MeetinSheep } from "./MeetinSheep";

describe("MeetinSheep roket (R3.0f)", () => {
  it("roketli uçuşta roket ve alev çizilir, pelerin ve arka toynak çizilmez", () => {
    const html = renderToStaticMarkup(
      <MeetinSheep anim="rocket" flightAngleDeg={30} />,
    );
    expect(html).toContain("a-rocket");
    expect(html).toContain("rocket-body");
    expect(html).toContain("rocket-flame");
    expect(html).toContain("rotate(30deg)");
    expect(html).not.toContain("super-cape-fabric");
    expect(html).not.toContain('class="legs"');
  });

  it("pelerinli uçuşta roket yoktur", () => {
    const html = renderToStaticMarkup(<MeetinSheep anim="super" />);
    expect(html).toContain("super-cape-fabric");
    expect(html).not.toContain("rocket-body");
    expect(html).not.toContain("rocket-flame");
  });

  it("azaltılmış harekette roket animasyonları da durur", () => {
    expect(css.length).toBeGreaterThan(1000);
    expect(css).toContain(".k-b2.a-rocket .rocket-flame");
    const reduced = css.slice(
      css.indexOf("@media (prefers-reduced-motion: reduce)"),
    );
    expect(reduced).toContain(".sheep *");
    expect(reduced).toContain("animation: none !important");
  });
});
