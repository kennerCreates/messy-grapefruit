import type { ViewportState, GridMode } from "../lib/types";
import { GRID_DOT_RADIUS, GRID_ZOOM_THRESHOLDS, GRID_DENSITY_SCREEN_PX } from "../lib/constants";

/**
 * Compute effective grid spacing that adapts to zoom level.
 * Finds the best subdivision so dots maintain roughly consistent screen-space density.
 */
function computeEffectiveGridSize(baseGridSize: number, zoom: number): number {
  // Walk through thresholds to find appropriate subdivision
  for (let i = GRID_ZOOM_THRESHOLDS.length - 1; i >= 0; i--) {
    if (zoom >= GRID_ZOOM_THRESHOLDS[i].zoom) {
      return baseGridSize * GRID_ZOOM_THRESHOLDS[i].gridDiv;
    }
  }
  // Below all thresholds, use the coarsest
  return baseGridSize * GRID_ZOOM_THRESHOLDS[0].gridDiv;
}

/**
 * Render the dot grid onto the canvas.
 * Only draws dots within the visible viewport for performance.
 */
export function renderGrid(
  ctx: CanvasRenderingContext2D,
  viewport: ViewportState,
  gridSize: number,
  gridMode: GridMode,
  canvasWidth: number,
  canvasHeight: number,
) {
  const { panX, panY, zoom } = viewport;

  const effectiveGrid = computeEffectiveGridSize(gridSize, zoom);
  if (effectiveGrid <= 0) return;

  // Compute visible canvas-space bounds from screen-space
  const screenW = ctx.canvas.width;
  const screenH = ctx.canvas.height;

  const left = -panX / zoom;
  const top = -panY / zoom;
  const right = (screenW - panX) / zoom;
  const bottom = (screenH - panY) / zoom;

  // Clamp to canvas bounds (no dots outside the sprite canvas)
  const startX = Math.max(0, Math.floor(left / effectiveGrid) * effectiveGrid);
  const startY = Math.max(0, Math.floor(top / effectiveGrid) * effectiveGrid);
  const endX = Math.min(canvasWidth, Math.ceil(right / effectiveGrid) * effectiveGrid);
  const endY = Math.min(canvasHeight, Math.ceil(bottom / effectiveGrid) * effectiveGrid);

  // Dot radius in canvas space (constant screen-space size)
  const dotRadius = GRID_DOT_RADIUS / zoom;

  ctx.save();

  // Grid dots are subtle
  ctx.fillStyle = "rgba(255, 255, 255, 0.15)";

  if (gridMode === "standard") {
    for (let x = startX; x <= endX; x += effectiveGrid) {
      for (let y = startY; y <= endY; y += effectiveGrid) {
        ctx.beginPath();
        ctx.arc(x, y, dotRadius, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  } else {
    // Isometric grid: offset every other row by half the grid spacing
    const rowHeight = effectiveGrid * (Math.sqrt(3) / 2);
    let row = 0;
    for (let y = startY; y <= endY; y += rowHeight) {
      const offsetX = row % 2 === 1 ? effectiveGrid / 2 : 0;
      for (let x = startX + offsetX; x <= endX; x += effectiveGrid) {
        ctx.beginPath();
        ctx.arc(x, y, dotRadius, 0, Math.PI * 2);
        ctx.fill();
      }
      row++;
    }
  }

  ctx.restore();
}
