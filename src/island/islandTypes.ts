import type { RefObject } from "react";
import type { MascotAnim } from "../mascot/mascotMachine";

// Ada penceresinin görünüm kipleri
export type IslandMode = "mascot" | "pill" | "suggest" | "search" | "add" | "fill" | "menu";

export interface FormFieldDto {
  name: string;
  options: string[] | null;
}

export interface FillIslandPayload {
  abbr: string;
  fields: FormFieldDto[];
  preview: string;
}

export interface PillNotice {
  ok: boolean;
  text: string;
}

// Her kipte çizilen koyunun ortak özellikleri (göz bebeği takibi aynı svgRef'i okur)
export interface SheepView {
  svgRef: RefObject<SVGSVGElement | null>;
  anim: MascotAnim;
  pupilOffset: { x: number; y: number };
  isVisible: boolean;
}
