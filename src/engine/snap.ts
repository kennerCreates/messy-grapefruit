import type { Vec2, GridMode } from "../lib/types";

/**
 * Snap a position to the nearest grid intersection.
 */
export function snapToGrid(pos: Vec2, gridSize: number, gridMode: GridMode): Vec2 {
  if (gridMode === "standard") {
    return {
      x: Math.round(pos.x / gridSize) * gridSize,
      y: Math.round(pos.y / gridSize) * gridSize,
    };
  }

  // Isometric mode (Phase 2 stub): for now, snap the same as standard.
  // Full implementation will snap to nearest isometric grid intersection.
  return {
    x: Math.round(pos.x / gridSize) * gridSize,
    y: Math.round(pos.y / gridSize) * gridSize,
  };
}
