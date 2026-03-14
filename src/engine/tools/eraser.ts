import type { CanvasTool } from "./base";
import type { Vec2, ViewportState } from "../../lib/types";

/**
 * Eraser tool stub (Phase 2).
 * Will support clicking on elements or vertices to delete them.
 */
export function createEraserTool(): CanvasTool {
  return {
    name: "eraser",

    onPointerDown(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: hit test and delete element or vertex
    },

    onPointerMove(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: highlight element under cursor for deletion preview
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2
    },

    render(_ctx: CanvasRenderingContext2D, _viewport: ViewportState) {
      // Phase 2: render eraser cursor and deletion preview
    },
  };
}
