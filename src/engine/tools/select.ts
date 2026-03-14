import type { CanvasTool } from "./base";
import type { Vec2, ViewportState } from "../../lib/types";

/**
 * Select tool stub (Phase 2).
 * Will support clicking to select elements, dragging to move, and box selection.
 */
export function createSelectTool(): CanvasTool {
  return {
    name: "select",

    onPointerDown(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: hit test elements, start selection or drag
    },

    onPointerMove(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: update drag position or selection box
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // Phase 2: finalize selection or move
    },

    onKeyDown(_e: KeyboardEvent) {
      // Phase 2: delete selected elements, etc.
    },

    render(_ctx: CanvasRenderingContext2D, _viewport: ViewportState) {
      // Phase 2: render selection highlights, handles, bounding box
    },
  };
}
