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

  // Dot radius in canvas space (constant screen-space size)
  const dotRadius = GRID_DOT_RADIUS / zoom;

  ctx.save();

  // Grid dots are subtle
  ctx.fillStyle = "rgba(255, 255, 255, 0.15)";

  if (gridMode === "standard") {
    // Clamp to canvas bounds (no dots outside the sprite canvas)
    const startX = Math.max(0, Math.floor(left / effectiveGrid) * effectiveGrid);
    const startY = Math.max(0, Math.floor(top / effectiveGrid) * effectiveGrid);
    const endX = Math.min(canvasWidth, Math.ceil(right / effectiveGrid) * effectiveGrid);
    const endY = Math.min(canvasHeight, Math.ceil(bottom / effectiveGrid) * effectiveGrid);

    for (let x = startX; x <= endX; x += effectiveGrid) {
      for (let y = startY; y <= endY; y += effectiveGrid) {
        ctx.beginPath();
        ctx.arc(x, y, dotRadius, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  } else {
    // Isometric grid: draw dots at isometric grid intersections (diamond pattern).
    // Uses 2:1 ratio (for every 2px horizontal, 1px vertical).
    //
    // The isometric basis vectors are:
    //   u = (effectiveGrid, effectiveGrid / 2)   -- right-down
    //   v = (-effectiveGrid, effectiveGrid / 2)   -- left-down
    //
    // Grid point at (i, j): x = (i - j) * effectiveGrid, y = (i + j) * effectiveGrid / 2

    const halfGrid = effectiveGrid / 2;

    // Determine range of i, j that produce visible points within canvas bounds
    // We need to cover all (i,j) such that:
    //   0 <= (i - j) * effectiveGrid <= canvasWidth
    //   0 <= (i + j) * halfGrid <= canvasHeight
    // AND the point is within the visible viewport

    // Convert viewport bounds to i,j range
    // x = (i-j)*effectiveGrid, y = (i+j)*halfGrid
    // i = (x/effectiveGrid + y/halfGrid) / 2
    // j = (y/halfGrid - x/effectiveGrid) / 2

    const visLeft = Math.max(0, left);
    const visTop = Math.max(0, top);
    const visRight = Math.min(canvasWidth, right);
    const visBottom = Math.min(canvasHeight, bottom);

    // Compute bounding range for i and j from all 4 corners of visible rect
    const corners = [
      { x: visLeft, y: visTop },
      { x: visRight, y: visTop },
      { x: visLeft, y: visBottom },
      { x: visRight, y: visBottom },
    ];

    let minI = Infinity, maxI = -Infinity;
    let minJ = Infinity, maxJ = -Infinity;

    for (const c of corners) {
      const ci = (c.x / effectiveGrid + c.y / halfGrid) / 2;
      const cj = (c.y / halfGrid - c.x / effectiveGrid) / 2;
      minI = Math.min(minI, ci);
      maxI = Math.max(maxI, ci);
      minJ = Math.min(minJ, cj);
      maxJ = Math.max(maxJ, cj);
    }

    // Add some padding and convert to integers
    const iStart = Math.floor(minI) - 1;
    const iEnd = Math.ceil(maxI) + 1;
    const jStart = Math.floor(minJ) - 1;
    const jEnd = Math.ceil(maxJ) + 1;

    for (let i = iStart; i <= iEnd; i++) {
      for (let j = jStart; j <= jEnd; j++) {
        const px = (i - j) * effectiveGrid;
        const py = (i + j) * halfGrid;

        // Skip points outside the canvas bounds
        if (px < 0 || px > canvasWidth || py < 0 || py > canvasHeight) continue;
        // Skip points outside the visible viewport
        if (px < left || px > right || py < top || py > bottom) continue;

        ctx.beginPath();
        ctx.arc(px, py, dotRadius, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  ctx.restore();
}
