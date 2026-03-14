import type { CanvasTool } from "./base";
import type {
  Vec2,
  ViewportState,
  PathVertex,
  StrokeElement,
  Element,
} from "../../lib/types";
import { generateId, generateAutoControlPoints, vec2Distance } from "../../lib/math";
import { snapToGrid } from "../snap";
import { findMergeTarget } from "../hit-test";
import { mergeElements, closeElement } from "../merge";
import {
  editorStore,
  setIsDrawing,
  addCurrentStrokeVertex,
  clearCurrentStroke,
  setCurrentStrokeVertices,
} from "../../stores/editor";
import { updateSprite, getActiveSprite } from "../../stores/project";
import { paletteColorCSS } from "../../stores/palette";
import { history } from "../../lib/history";
import { MERGE_THRESHOLD } from "../../lib/constants";

/** Current cursor position in canvas space, for preview rendering */
let cursorCanvasPos: Vec2 = { x: 0, y: 0 };

/** Current merge preview target, if any */
let mergePreview: { elementId: string; vertexId: string } | null = null;

/**
 * Get elements on the active layer of the active sprite.
 */
function getActiveLayerElements(): Element[] {
  const sprite = getActiveSprite();
  if (!sprite || sprite.layers.length === 0) return [];
  // Use the first layer for now; layer selection will be added in Phase 2
  return sprite.layers[0].elements;
}

/**
 * Finalize the current stroke: create a StrokeElement, handle auto-merge,
 * and add to the active layer via the history system.
 */
function finishStroke() {
  const vertices = [...editorStore.currentStrokeVertices];
  if (vertices.length < 2) {
    clearCurrentStroke();
    return;
  }

  const sprite = getActiveSprite();
  if (!sprite) {
    clearCurrentStroke();
    return;
  }

  // Auto-generate bezier control points if curve mode is on
  const finalVertices = editorStore.curveMode
    ? generateAutoControlPoints(vertices, false)
    : vertices;

  let newElement: StrokeElement = {
    id: generateId(),
    type: "stroke",
    vertices: finalVertices,
    closed: false,
    strokeWidth: editorStore.strokeWidth,
    strokeColorIndex: editorStore.activeColorIndex,
    fillColorIndex: 0, // no fill by default
    position: { x: 0, y: 0 },
    rotation: 0,
    scale: { x: 1, y: 1 },
    origin: { x: 0, y: 0 },
    visible: true,
  };

  const layerElements = getActiveLayerElements();
  const spriteId = sprite.id;

  // Check for auto-merge at first vertex
  const firstVertexPos = finalVertices[0].pos;
  const mergeFirst = findMergeTarget(firstVertexPos, layerElements, null, MERGE_THRESHOLD * editorStore.gridSize);

  // Check for auto-merge at last vertex
  const lastVertexPos = finalVertices[finalVertices.length - 1].pos;
  const mergeLast = findMergeTarget(lastVertexPos, layerElements, null, MERGE_THRESHOLD * editorStore.gridSize);

  // Check for self-close: last vertex near first vertex of same stroke
  const selfClose =
    finalVertices.length >= 3 &&
    vec2Distance(firstVertexPos, lastVertexPos) <= MERGE_THRESHOLD * editorStore.gridSize;

  if (selfClose) {
    newElement = closeElement(newElement);
  }

  // Execute via history for undo/redo
  history.execute({
    description: "Draw stroke",
    execute: () => {
      updateSprite(spriteId, (s) => {
        if (s.layers.length === 0) return;

        if (mergeFirst && mergeLast && mergeFirst.elementId === mergeLast.elementId) {
          // Both ends merge to the same element: merge and close
          const targetIdx = s.layers[0].elements.findIndex(
            (e) => e.id === mergeFirst.elementId,
          );
          if (targetIdx >= 0) {
            const target = s.layers[0].elements[targetIdx] as StrokeElement;
            let merged = mergeElements(target, newElement);
            merged = closeElement(merged);
            s.layers[0].elements[targetIdx] = merged;
          } else {
            s.layers[0].elements.push(newElement);
          }
        } else if (mergeLast) {
          // Last vertex merges into existing element
          const targetIdx = s.layers[0].elements.findIndex(
            (e) => e.id === mergeLast.elementId,
          );
          if (targetIdx >= 0) {
            const target = s.layers[0].elements[targetIdx] as StrokeElement;
            s.layers[0].elements[targetIdx] = mergeElements(target, newElement);
          } else {
            s.layers[0].elements.push(newElement);
          }
        } else if (mergeFirst) {
          // First vertex merges into existing element
          const targetIdx = s.layers[0].elements.findIndex(
            (e) => e.id === mergeFirst.elementId,
          );
          if (targetIdx >= 0) {
            const target = s.layers[0].elements[targetIdx] as StrokeElement;
            // Reverse: the new element's start connects to target's end
            const reversed: StrokeElement = {
              ...newElement,
              vertices: [...newElement.vertices].reverse(),
            };
            s.layers[0].elements[targetIdx] = mergeElements(target, reversed);
          } else {
            s.layers[0].elements.push(newElement);
          }
        } else {
          s.layers[0].elements.push(newElement);
        }
      });
    },
    undo: () => {
      updateSprite(spriteId, (s) => {
        if (s.layers.length === 0) return;
        // Simple undo: remove the last added element
        // For merges, this is a simplification; full undo would restore the original
        const idx = s.layers[0].elements.findIndex((e) => e.id === newElement.id);
        if (idx >= 0) {
          s.layers[0].elements.splice(idx, 1);
        }
      });
    },
  });

  clearCurrentStroke();
}

export function createLineTool(): CanvasTool {
  return {
    name: "line",

    onPointerDown(_e: PointerEvent, canvasPos: Vec2) {
      const snapped = snapToGrid(canvasPos, editorStore.gridSize, editorStore.gridMode);

      if (!editorStore.isDrawing) {
        // Start a new stroke
        setIsDrawing(true);
        const vertex: PathVertex = {
          id: generateId(),
          pos: snapped,
        };
        setCurrentStrokeVertices([vertex]);
      } else {
        // Place the next vertex
        const vertices = editorStore.currentStrokeVertices;

        // Check for self-close: clicking near the first vertex
        if (
          vertices.length >= 3 &&
          vec2Distance(snapped, vertices[0].pos) <=
            MERGE_THRESHOLD * editorStore.gridSize
        ) {
          // Close the shape
          finishStroke();
          return;
        }

        const vertex: PathVertex = {
          id: generateId(),
          pos: snapped,
        };
        addCurrentStrokeVertex(vertex);
      }
    },

    onPointerMove(_e: PointerEvent, canvasPos: Vec2) {
      cursorCanvasPos = canvasPos;

      // Update merge preview
      const snapped = snapToGrid(canvasPos, editorStore.gridSize, editorStore.gridMode);
      const layerElements = getActiveLayerElements();
      mergePreview = findMergeTarget(
        snapped,
        layerElements,
        null,
        MERGE_THRESHOLD * editorStore.gridSize,
      );

      // Check self-close preview
      if (editorStore.isDrawing && editorStore.currentStrokeVertices.length >= 3) {
        const firstPos = editorStore.currentStrokeVertices[0].pos;
        if (
          vec2Distance(snapped, firstPos) <=
          MERGE_THRESHOLD * editorStore.gridSize
        ) {
          mergePreview = {
            elementId: "__self__",
            vertexId: editorStore.currentStrokeVertices[0].id,
          };
        }
      }
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // Line tool places vertices on pointer down, not up
    },

    onDoubleClick(_e: MouseEvent, _canvasPos: Vec2) {
      // Finish the current stroke
      if (editorStore.isDrawing) {
        finishStroke();
      }
    },

    onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && editorStore.isDrawing) {
        // Cancel current stroke
        clearCurrentStroke();
      } else if (e.key === "Enter" && editorStore.isDrawing) {
        // Finish current stroke
        finishStroke();
      }
    },

    render(ctx: CanvasRenderingContext2D, viewport: ViewportState) {
      const vertices = editorStore.currentStrokeVertices;
      if (vertices.length === 0 && !mergePreview) return;

      const strokeColor = paletteColorCSS(editorStore.activeColorIndex, "#ffffff");

      // Draw in-progress stroke preview
      if (vertices.length > 0) {
        ctx.save();
        ctx.strokeStyle = strokeColor;
        ctx.lineWidth = editorStore.strokeWidth;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";
        ctx.globalAlpha = 0.7;

        // Draw placed segments
        ctx.beginPath();
        ctx.moveTo(vertices[0].pos.x, vertices[0].pos.y);

        if (editorStore.curveMode && vertices.length >= 2) {
          // Generate temporary control points for preview
          const tempVertices = generateAutoControlPoints(vertices, false);
          for (let i = 1; i < tempVertices.length; i++) {
            const prev = tempVertices[i - 1];
            const curr = tempVertices[i];
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
        } else {
          for (let i = 1; i < vertices.length; i++) {
            ctx.lineTo(vertices[i].pos.x, vertices[i].pos.y);
          }
        }
        ctx.stroke();

        // Draw preview line from last vertex to cursor
        if (editorStore.isDrawing) {
          ctx.setLineDash([4 / viewport.zoom, 4 / viewport.zoom]);
          ctx.globalAlpha = 0.4;
          ctx.beginPath();
          const last = vertices[vertices.length - 1];
          ctx.moveTo(last.pos.x, last.pos.y);
          ctx.lineTo(cursorCanvasPos.x, cursorCanvasPos.y);
          ctx.stroke();
          ctx.setLineDash([]);
        }

        // Draw vertex handles
        ctx.globalAlpha = 1;
        const handleRadius = 4 / viewport.zoom;
        for (const v of vertices) {
          ctx.beginPath();
          ctx.arc(v.pos.x, v.pos.y, handleRadius, 0, Math.PI * 2);
          ctx.fillStyle = strokeColor;
          ctx.fill();
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 1 / viewport.zoom;
          ctx.stroke();
        }

        ctx.restore();
      }

      // Draw merge preview indicator
      if (mergePreview) {
        ctx.save();
        const targetVertex = findMergeTargetVertex();
        if (targetVertex) {
          const indicatorRadius = 8 / viewport.zoom;
          ctx.beginPath();
          ctx.arc(targetVertex.x, targetVertex.y, indicatorRadius, 0, Math.PI * 2);
          ctx.strokeStyle = "#00ff88";
          ctx.lineWidth = 2 / viewport.zoom;
          ctx.setLineDash([3 / viewport.zoom, 3 / viewport.zoom]);
          ctx.stroke();
          ctx.setLineDash([]);

          // Inner filled circle
          ctx.beginPath();
          ctx.arc(targetVertex.x, targetVertex.y, indicatorRadius * 0.4, 0, Math.PI * 2);
          ctx.fillStyle = "rgba(0, 255, 136, 0.5)";
          ctx.fill();
        }
        ctx.restore();
      }
    },
  };
}

/**
 * Find the world-space position of the current merge preview target vertex.
 */
function findMergeTargetVertex(): Vec2 | null {
  if (!mergePreview) return null;

  if (mergePreview.elementId === "__self__") {
    // Self-close: the first vertex of the current stroke
    const vertices = editorStore.currentStrokeVertices;
    if (vertices.length > 0) {
      return vertices[0].pos;
    }
    return null;
  }

  const layerElements = getActiveLayerElements();
  for (const el of layerElements) {
    if (el.type !== "stroke") continue;
    if (el.id === mergePreview.elementId) {
      for (const v of el.vertices) {
        if (v.id === mergePreview.vertexId) {
          return v.pos;
        }
      }
    }
  }
  return null;
}

