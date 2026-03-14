import type {
  StrokeElement,
  ViewportState,
  Skin,
  SkinOverride,
  PaletteColor,
  ToolType,
} from "../lib/types";
import { THEMES } from "../lib/constants";
import { editorStore } from "../stores/editor";
import { projectStore, getActiveSprite } from "../stores/project";
import { paletteColorCSS, paletteColors } from "../stores/palette";
import { renderGrid } from "./grid";
import type { CanvasTool } from "./tools/base";
import { createLineTool } from "./tools/line";
import { createSelectTool } from "./tools/select";
import { createFillTool } from "./tools/fill";
import { createEraserTool } from "./tools/eraser";

export interface CanvasRenderer {
  /** Start the render loop */
  start(): void;
  /** Stop the render loop and clean up */
  destroy(): void;
  /** Get the current active tool instance */
  getActiveTool(): CanvasTool | null;
  /** Get a tool by name */
  getTool(name: ToolType): CanvasTool;
  /** Get the CSS cursor for the active tool */
  getCursor(): string;
}

/**
 * Resolve a palette color index to a CSS color string, applying skin overrides if present.
 */
function resolveColor(
  colorIndex: number,
  palette: PaletteColor[],
  fallback: string = "transparent",
): string {
  if (colorIndex < 0 || colorIndex >= palette.length) return fallback;
  const c = palette[colorIndex];
  return `rgba(${c.r}, ${c.g}, ${c.b}, ${c.a / 255})`;
}

/**
 * Find a skin override for a given element, if an active skin is set.
 */
function findSkinOverride(
  elementId: string,
  skin: Skin | null,
): SkinOverride | null {
  if (!skin) return null;
  return skin.overrides.find((o) => o.elementId === elementId) ?? null;
}

/**
 * Build and stroke/fill a StrokeElement's path on the canvas.
 */
function renderStrokeElement(
  ctx: CanvasRenderingContext2D,
  element: StrokeElement,
  palette: PaletteColor[],
  skin: Skin | null,
  isSelected: boolean,
  selectedVertexIds: string[],
  viewport: ViewportState,
) {
  if (!element.visible) return;

  const verts = element.vertices;
  if (verts.length === 0) return;

  // Resolve colors (apply skin override if active)
  const override = findSkinOverride(element.id, skin);
  const strokeColorIdx = override?.strokeColorIndex ?? element.strokeColorIndex;
  const fillColorIdx = override?.fillColorIndex ?? element.fillColorIndex;
  const strokeWidth = override?.strokeWidth ?? element.strokeWidth;

  const strokeColor = resolveColor(strokeColorIdx, palette, "#ffffff");
  const fillColor = resolveColor(fillColorIdx, palette, "transparent");

  ctx.save();

  // Apply element transform
  ctx.translate(element.position.x, element.position.y);
  ctx.rotate(element.rotation);
  ctx.scale(element.scale.x, element.scale.y);
  ctx.translate(-element.origin.x, -element.origin.y);

  // Build path
  ctx.beginPath();
  ctx.moveTo(verts[0].pos.x, verts[0].pos.y);

  for (let i = 1; i < verts.length; i++) {
    const prev = verts[i - 1];
    const curr = verts[i];

    if (prev.cp1 && curr.cp2) {
      ctx.bezierCurveTo(
        prev.cp1.x,
        prev.cp1.y,
        curr.cp2.x,
        curr.cp2.y,
        curr.pos.x,
        curr.pos.y,
      );
    } else {
      ctx.lineTo(curr.pos.x, curr.pos.y);
    }
  }

  // Close path back to first vertex if closed
  if (element.closed && verts.length > 1) {
    const last = verts[verts.length - 1];
    const first = verts[0];
    if (last.cp1 && first.cp2) {
      ctx.bezierCurveTo(
        last.cp1.x,
        last.cp1.y,
        first.cp2.x,
        first.cp2.y,
        first.pos.x,
        first.pos.y,
      );
    } else {
      ctx.closePath();
    }
  }

  // Fill if closed and fill color is set
  if (element.closed && fillColorIdx > 0) {
    ctx.fillStyle = fillColor;
    ctx.fill();
  }

  // Stroke
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = strokeWidth;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.stroke();

  // Draw selection highlights
  if (isSelected) {
    ctx.strokeStyle = "#00aaff";
    ctx.lineWidth = strokeWidth + 2 / viewport.zoom;
    ctx.globalAlpha = 0.4;
    ctx.stroke();
    ctx.globalAlpha = 1;

    // Draw vertex handles
    const handleRadius = 4 / viewport.zoom;
    const cpHandleRadius = 3 / viewport.zoom;

    for (const v of verts) {
      const isVertexSelected = selectedVertexIds.includes(v.id);

      // Draw control point handles and lines
      if (v.cp1) {
        ctx.beginPath();
        ctx.moveTo(v.pos.x, v.pos.y);
        ctx.lineTo(v.cp1.x, v.cp1.y);
        ctx.strokeStyle = "rgba(0, 170, 255, 0.3)";
        ctx.lineWidth = 1 / viewport.zoom;
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(v.cp1.x, v.cp1.y, cpHandleRadius, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(0, 170, 255, 0.6)";
        ctx.fill();
      }

      if (v.cp2) {
        ctx.beginPath();
        ctx.moveTo(v.pos.x, v.pos.y);
        ctx.lineTo(v.cp2.x, v.cp2.y);
        ctx.strokeStyle = "rgba(0, 170, 255, 0.3)";
        ctx.lineWidth = 1 / viewport.zoom;
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(v.cp2.x, v.cp2.y, cpHandleRadius, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(0, 170, 255, 0.6)";
        ctx.fill();
      }

      // Draw vertex handle
      ctx.beginPath();
      ctx.arc(v.pos.x, v.pos.y, handleRadius, 0, Math.PI * 2);
      ctx.fillStyle = isVertexSelected ? "#ffffff" : "#00aaff";
      ctx.fill();
      ctx.strokeStyle = isVertexSelected ? "#00aaff" : "#ffffff";
      ctx.lineWidth = 1 / viewport.zoom;
      ctx.stroke();
    }
  }

  ctx.restore();
}

/**
 * Render the canvas boundary as a dashed rectangle.
 */
function renderCanvasBoundary(
  ctx: CanvasRenderingContext2D,
  canvasWidth: number,
  canvasHeight: number,
  viewport: ViewportState,
) {
  ctx.save();
  ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";
  ctx.lineWidth = 1 / viewport.zoom;
  ctx.setLineDash([6 / viewport.zoom, 4 / viewport.zoom]);
  ctx.strokeRect(0, 0, canvasWidth, canvasHeight);
  ctx.setLineDash([]);
  ctx.restore();
}

/**
 * Create the main canvas renderer.
 */
export function createCanvasRenderer(canvas: HTMLCanvasElement): CanvasRenderer {
  const ctx = canvas.getContext("2d")!;
  let animFrameId: number | null = null;
  let running = false;

  // Create tool instances
  const tools: Record<ToolType, CanvasTool> = {
    line: createLineTool(),
    select: createSelectTool(),
    fill: createFillTool(),
    eraser: createEraserTool(),
  };

  function getActiveTool(): CanvasTool | null {
    return tools[editorStore.activeTool] ?? null;
  }

  function getTool(name: ToolType): CanvasTool {
    return tools[name];
  }

  function getCursor(): string {
    const tool = getActiveTool();
    return tool?.cursor ?? "default";
  }

  /**
   * Main render function, called each frame.
   */
  function render() {
    const { width, height } = canvas;
    const viewport = editorStore.viewport;

    // Clear the canvas
    ctx.clearRect(0, 0, width, height);

    // Fill background
    const theme = THEMES.dark; // TODO: reactive theme selection
    ctx.fillStyle = theme.bg;
    ctx.fillRect(0, 0, width, height);

    // Apply viewport transform
    ctx.save();
    ctx.translate(viewport.panX, viewport.panY);
    ctx.scale(viewport.zoom, viewport.zoom);

    const sprite = getActiveSprite();
    const spriteWidth = sprite?.canvasWidth ?? 256;
    const spriteHeight = sprite?.canvasHeight ?? 256;

    // Draw canvas background (sprite area)
    if (sprite) {
      const bgColor = paletteColorCSS(sprite.backgroundColorIndex, theme.panels);
      ctx.fillStyle = bgColor;
      ctx.fillRect(0, 0, spriteWidth, spriteHeight);
    }

    // Draw grid
    if (editorStore.showGrid) {
      renderGrid(ctx, viewport, editorStore.gridSize, editorStore.gridMode, spriteWidth, spriteHeight);
    }

    // Draw canvas boundary
    renderCanvasBoundary(ctx, spriteWidth, spriteHeight, viewport);

    // Draw all visible elements
    if (sprite) {
      const palette = paletteColors();
      // Find active skin if any
      const activeSkinId = projectStore.project?.sprites.find(
        (s) => s.id === sprite.id,
      )?.selectedSkinId;
      const activeSkin = activeSkinId
        ? sprite.skins.find((s) => s.id === activeSkinId) ?? null
        : null;

      for (const layer of sprite.layers) {
        if (!layer.visible) continue;

        for (const element of layer.elements) {
          if (element.type === "stroke") {
            const isSelected = editorStore.selectedElementIds.includes(element.id);
            renderStrokeElement(
              ctx,
              element,
              palette,
              activeSkin,
              isSelected,
              editorStore.selectedVertexIds,
              viewport,
            );
          }
          // IK targets: Phase 2 rendering
        }
      }
    }

    // Render active tool overlay (preview lines, handles, etc.)
    const tool = getActiveTool();
    if (tool?.render) {
      tool.render(ctx, viewport);
    }

    ctx.restore(); // viewport transform

    // Continue the render loop
    if (running) {
      animFrameId = requestAnimationFrame(render);
    }
  }

  return {
    start() {
      if (running) return;
      running = true;
      animFrameId = requestAnimationFrame(render);
    },

    destroy() {
      running = false;
      if (animFrameId !== null) {
        cancelAnimationFrame(animFrameId);
        animFrameId = null;
      }
    },

    getActiveTool,
    getTool,
    getCursor,
  };
}
