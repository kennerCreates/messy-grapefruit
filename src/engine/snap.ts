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

  // Isometric grid uses a 2:1 pixel ratio (26.57 degree angle).
  // The isometric axes are at +/- 26.57 degrees from horizontal.
  // We transform to isometric space, snap, then transform back.
  //
  // Isometric grid basis vectors (2:1 ratio):
  //   u = (gridSize, gridSize / 2)   -- right-down axis
  //   v = (-gridSize, gridSize / 2)  -- left-down axis
  //
  // To snap, we convert the position to (u, v) coordinates,
  // round to nearest integer, and convert back.

  const halfGrid = gridSize / 2;

  // Transform to isometric coordinates:
  // pos = u_coord * u_vec + v_coord * v_vec
  // pos.x = u_coord * gridSize + v_coord * (-gridSize)
  // pos.y = u_coord * halfGrid + v_coord * halfGrid
  //
  // Solving:
  // u_coord = (pos.x / gridSize + pos.y / halfGrid) / 2
  // v_coord = (pos.y / halfGrid - pos.x / gridSize) / 2

  const uCoord = (pos.x / gridSize + pos.y / halfGrid) / 2;
  const vCoord = (pos.y / halfGrid - pos.x / gridSize) / 2;

  // Snap to nearest integer in isometric space
  const uSnapped = Math.round(uCoord);
  const vSnapped = Math.round(vCoord);

  // Convert back to cartesian
  return {
    x: (uSnapped - vSnapped) * gridSize,
    y: (uSnapped + vSnapped) * halfGrid,
  };
}
