import type { CanvasTool } from "./base";
import type { Vec2, ViewportState } from "../../lib/types";

/**
 * Fill tool stub (Phase 2).
 * Will support clicking on a closed element to set its fill color.
 */
export function createFillTool(): CanvasTool {
  return {
    name: "fill",

    onPointerDown(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: hit test closed elements and apply fill color
    },

    onPointerMove(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: highlight element under cursor
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2
    },

    render(_ctx: CanvasRenderingContext2D, _viewport: ViewportState) {
      // Phase 2: render fill preview highlight
    },
  };
}
