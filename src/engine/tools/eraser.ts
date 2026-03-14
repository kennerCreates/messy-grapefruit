import type { Vec2, StrokeElement, Element, ViewportState } from "../../lib/types";
import { editorStore } from "../../stores/editor";
import { getActiveSprite, updateSprite } from "../../stores/project";
import { history } from "../../lib/history";
import { generateId } from "../../lib/math";
import { hitTestElement, hitTestVertex } from "../hit-test";
import { MERGE_THRESHOLD } from "../../lib/constants";
import type { CanvasTool } from "./base";

/** Stored reference to the canvas context for hit testing */
let ctx: CanvasRenderingContext2D | null = null;

/** Currently hovered element/vertex for preview */
let hoveredElementId: string | null = null;
let hoveredVertexId: string | null = null;

/**
 * Get all elements from visible, unlocked layers.
 */
function getAllEditableElements(): {
  element: Element;
  layerId: string;
  layerIndex: number;
}[] {
  const sprite = getActiveSprite();
  if (!sprite) return [];
  const results: { element: Element; layerId: string; layerIndex: number }[] = [];
  for (let li = 0; li < sprite.layers.length; li++) {
    const layer = sprite.layers[li];
    if (!layer.visible || layer.locked) continue;
    for (const el of layer.elements) {
      results.push({ element: el, layerId: layer.id, layerIndex: li });
    }
  }
  return results;
}

/**
 * Hit test vertices across all editable elements.
 * Returns the element and vertex ID if found.
 */
function hitTestVertexAll(pos: Vec2): {
  element: StrokeElement;
  vertexId: string;
  layerIndex: number;
} | null {
  const threshold = MERGE_THRESHOLD * editorStore.gridSize;
  const allElements = getAllEditableElements();
  // Test in reverse for topmost priority
  for (let i = allElements.length - 1; i >= 0; i--) {
    const entry = allElements[i];
    if (entry.element.type !== "stroke") continue;
    const strokeEl = entry.element as StrokeElement;
    const vertexId = hitTestVertex(pos, strokeEl.vertices, threshold);
    if (vertexId) {
      return { element: strokeEl, vertexId, layerIndex: entry.layerIndex };
    }
  }
  return null;
}

/**
 * Hit test elements across all editable elements.
 */
function hitTestElementAll(pos: Vec2): {
  element: StrokeElement;
  layerIndex: number;
} | null {
  if (!ctx) return null;
  const allElements = getAllEditableElements();
  for (let i = allElements.length - 1; i >= 0; i--) {
    const entry = allElements[i];
    if (entry.element.type !== "stroke") continue;
    const strokeEl = entry.element as StrokeElement;
    if (hitTestElement(pos, strokeEl, ctx)) {
      return { element: strokeEl, layerIndex: entry.layerIndex };
    }
  }
  return null;
}

export function createEraserTool(): CanvasTool {
  return {
    name: "eraser",
    cursor: "crosshair",

    onPointerDown(_e: PointerEvent, canvasPos: Vec2) {
      const sprite = getActiveSprite();
      if (!sprite) return;
      const spriteId = sprite.id;

      // Priority 1: Hit test vertices
      const vertexHit = hitTestVertexAll(canvasPos);
      if (vertexHit) {
        const { element, vertexId, layerIndex } = vertexHit;
        const elementId = element.id;
        const originalElement: StrokeElement = JSON.parse(JSON.stringify(element));
        const vertexIndex = element.vertices.findIndex((v) => v.id === vertexId);

        if (vertexIndex < 0) return;

        if (element.vertices.length <= 2) {
          // Element has only 1-2 vertices: delete the entire element
          history.execute({
            description: "Delete element (too few vertices)",
            execute: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  s.layers[layerIndex].elements = s.layers[layerIndex].elements.filter(
                    (el) => el.id !== elementId,
                  );
                }
              });
            },
            undo: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  s.layers[layerIndex].elements.push(
                    JSON.parse(JSON.stringify(originalElement)),
                  );
                }
              });
            },
          });
        } else if (
          !element.closed &&
          vertexIndex > 0 &&
          vertexIndex < element.vertices.length - 1
        ) {
          // Vertex is in the middle of an open path: split into two elements
          const beforeVertices = element.vertices.slice(0, vertexIndex);
          const afterVertices = element.vertices.slice(vertexIndex + 1);

          const element1: StrokeElement = {
            ...originalElement,
            id: generateId(),
            vertices: beforeVertices.map((v) => ({ ...v, id: generateId() })),
            closed: false,
          };

          const element2: StrokeElement = {
            ...originalElement,
            id: generateId(),
            vertices: afterVertices.map((v) => ({ ...v, id: generateId() })),
            closed: false,
          };

          history.execute({
            description: "Split element by removing vertex",
            execute: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  const layer = s.layers[layerIndex];
                  const idx = layer.elements.findIndex((el) => el.id === elementId);
                  if (idx >= 0) {
                    layer.elements.splice(
                      idx,
                      1,
                      JSON.parse(JSON.stringify(element1)),
                      JSON.parse(JSON.stringify(element2)),
                    );
                  }
                }
              });
            },
            undo: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  const layer = s.layers[layerIndex];
                  // Remove the two split elements and restore original
                  layer.elements = layer.elements.filter(
                    (el) => el.id !== element1.id && el.id !== element2.id,
                  );
                  layer.elements.push(JSON.parse(JSON.stringify(originalElement)));
                }
              });
            },
          });
        } else {
          // Vertex is at an endpoint (or path is closed): just remove it
          const newVertices = [...element.vertices];
          newVertices.splice(vertexIndex, 1);

          // If it was a closed path, open it since we removed a vertex
          const newClosed = element.closed && newVertices.length >= 3
            ? element.closed
            : false;

          history.execute({
            description: "Remove vertex",
            execute: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  const layer = s.layers[layerIndex];
                  const idx = layer.elements.findIndex((el) => el.id === elementId);
                  if (idx >= 0) {
                    const el = layer.elements[idx] as StrokeElement;
                    el.vertices = JSON.parse(JSON.stringify(newVertices));
                    el.closed = newClosed;
                  }
                }
              });
            },
            undo: () => {
              updateSprite(spriteId, (s) => {
                if (layerIndex < s.layers.length) {
                  const layer = s.layers[layerIndex];
                  const idx = layer.elements.findIndex((el) => el.id === elementId);
                  if (idx >= 0) {
                    layer.elements[idx] = JSON.parse(JSON.stringify(originalElement));
                  }
                }
              });
            },
          });
        }
        return;
      }

      // Priority 2: Hit test elements (delete entire element)
      const elementHit = hitTestElementAll(canvasPos);
      if (elementHit) {
        const { element, layerIndex } = elementHit;
        const elementId = element.id;
        const originalElement: StrokeElement = JSON.parse(JSON.stringify(element));

        history.execute({
          description: "Erase element",
          execute: () => {
            updateSprite(spriteId, (s) => {
              if (layerIndex < s.layers.length) {
                s.layers[layerIndex].elements = s.layers[layerIndex].elements.filter(
                  (el) => el.id !== elementId,
                );
              }
            });
          },
          undo: () => {
            updateSprite(spriteId, (s) => {
              if (layerIndex < s.layers.length) {
                s.layers[layerIndex].elements.push(
                  JSON.parse(JSON.stringify(originalElement)),
                );
              }
            });
          },
        });
      }
    },

    onPointerMove(_e: PointerEvent, canvasPos: Vec2) {
      // Update hover preview
      const vertexHit = hitTestVertexAll(canvasPos);
      if (vertexHit) {
        hoveredElementId = vertexHit.element.id;
        hoveredVertexId = vertexHit.vertexId;
        return;
      }

      const elementHit = hitTestElementAll(canvasPos);
      if (elementHit) {
        hoveredElementId = elementHit.element.id;
        hoveredVertexId = null;
      } else {
        hoveredElementId = null;
        hoveredVertexId = null;
      }
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // No action on pointer up
    },

    render(renderCtx: CanvasRenderingContext2D, viewport: ViewportState) {
      // Store ctx reference for hit testing
      ctx = renderCtx;

      // Draw deletion preview highlight
      if (hoveredVertexId && hoveredElementId) {
        // Find the vertex position and draw an X over it
        const sprite = getActiveSprite();
        if (sprite) {
          for (const layer of sprite.layers) {
            for (const el of layer.elements) {
              if (el.type === "stroke" && el.id === hoveredElementId) {
                const v = el.vertices.find((v) => v.id === hoveredVertexId);
                if (v) {
                  const size = 6 / viewport.zoom;
                  renderCtx.save();
                  renderCtx.strokeStyle = "#ff4444";
                  renderCtx.lineWidth = 2 / viewport.zoom;
                  renderCtx.beginPath();
                  renderCtx.moveTo(v.pos.x - size, v.pos.y - size);
                  renderCtx.lineTo(v.pos.x + size, v.pos.y + size);
                  renderCtx.moveTo(v.pos.x + size, v.pos.y - size);
                  renderCtx.lineTo(v.pos.x - size, v.pos.y + size);
                  renderCtx.stroke();

                  // Circle around vertex
                  renderCtx.beginPath();
                  renderCtx.arc(v.pos.x, v.pos.y, size * 1.5, 0, Math.PI * 2);
                  renderCtx.strokeStyle = "rgba(255, 68, 68, 0.5)";
                  renderCtx.stroke();
                  renderCtx.restore();
                }
              }
            }
          }
        }
      } else if (hoveredElementId) {
        // Highlight the whole element in red
        const sprite = getActiveSprite();
        if (sprite) {
          for (const layer of sprite.layers) {
            for (const el of layer.elements) {
              if (el.type === "stroke" && el.id === hoveredElementId) {
                // Redraw the element path in red
                const verts = el.vertices;
                if (verts.length === 0) continue;

                renderCtx.save();
                renderCtx.translate(el.position.x, el.position.y);
                renderCtx.rotate(el.rotation);
                renderCtx.scale(el.scale.x, el.scale.y);
                renderCtx.translate(-el.origin.x, -el.origin.y);

                renderCtx.beginPath();
                renderCtx.moveTo(verts[0].pos.x, verts[0].pos.y);
                for (let i = 1; i < verts.length; i++) {
                  const prev = verts[i - 1];
                  const curr = verts[i];
                  if (prev.cp1 && curr.cp2) {
                    renderCtx.bezierCurveTo(
                      prev.cp1.x, prev.cp1.y,
                      curr.cp2.x, curr.cp2.y,
                      curr.pos.x, curr.pos.y,
                    );
                  } else {
                    renderCtx.lineTo(curr.pos.x, curr.pos.y);
                  }
                }
                if (el.closed) renderCtx.closePath();

                renderCtx.strokeStyle = "rgba(255, 68, 68, 0.6)";
                renderCtx.lineWidth = (el.strokeWidth + 3) / viewport.zoom;
                renderCtx.stroke();
                renderCtx.restore();
              }
            }
          }
        }
      }
    },
  };
}
