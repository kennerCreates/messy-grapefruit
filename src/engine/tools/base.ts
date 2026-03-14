import type { ToolType, Vec2, ViewportState } from "../../lib/types";

export interface CanvasTool {
  name: ToolType;
  onPointerDown(e: PointerEvent, canvasPos: Vec2): void;
  onPointerMove(e: PointerEvent, canvasPos: Vec2): void;
  onPointerUp(e: PointerEvent, canvasPos: Vec2): void;
  onDoubleClick?(e: MouseEvent, canvasPos: Vec2): void;
  onKeyDown?(e: KeyboardEvent): void;
  render?(ctx: CanvasRenderingContext2D, viewport: ViewportState): void;
}
