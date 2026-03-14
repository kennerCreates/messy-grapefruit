import type { Vec2, StrokeElement, Element, ViewportState } from "../../lib/types";
import { editorStore } from "../../stores/editor";
import { getActiveSprite, updateSprite } from "../../stores/project";
import { history } from "../../lib/history";
import { hitTestElement } from "../hit-test";
import type { CanvasTool } from "./base";

/** Stored reference to the canvas context for hit testing */
let ctx: CanvasRenderingContext2D | null = null;

/** Currently hovered element for preview, if any */
let hoveredElementId: string | null = null;

/**
 * Get all elements from visible, unlocked layers.
 */
function getAllEditableElements(): { element: Element; layerIndex: number }[] {
  const sprite = getActiveSprite();
  if (!sprite) return [];
  const results: { element: Element; layerIndex: number }[] = [];
  for (let li = 0; li < sprite.layers.length; li++) {
    const layer = sprite.layers[li];
    if (!layer.visible || layer.locked) continue;
    for (const el of layer.elements) {
      results.push({ element: el, layerIndex: li });
    }
  }
  return results;
}

/**
 * Hit test all editable elements at a position, returning the topmost closed StrokeElement.
 */
function hitTestClosedElement(pos: Vec2): StrokeElement | null {
  if (!ctx) return null;
  const allElements = getAllEditableElements();
  for (let i = allElements.length - 1; i >= 0; i--) {
    const entry = allElements[i];
    if (entry.element.type === "stroke") {
      const strokeEl = entry.element as StrokeElement;
      if (strokeEl.closed && hitTestElement(pos, strokeEl, ctx)) {
        return strokeEl;
      }
    }
  }
  return null;
}

/**
 * Hit test all editable elements (closed or not).
 */
function hitTestAny(pos: Vec2): StrokeElement | null {
  if (!ctx) return null;
  const allElements = getAllEditableElements();
  for (let i = allElements.length - 1; i >= 0; i--) {
    const entry = allElements[i];
    if (entry.element.type === "stroke") {
      const strokeEl = entry.element as StrokeElement;
      if (hitTestElement(pos, strokeEl, ctx)) {
        return strokeEl;
      }
    }
  }
  return null;
}

export function createFillTool(): CanvasTool {
  return {
    name: "fill",
    cursor: "crosshair",

    onPointerDown(_e: PointerEvent, canvasPos: Vec2) {
      const sprite = getActiveSprite();
      if (!sprite) return;
      const spriteId = sprite.id;
      const colorIndex = editorStore.activeColorIndex;

      // Try to hit a closed StrokeElement first
      const closedHit = hitTestClosedElement(canvasPos);
      if (closedHit) {
        const elementId = closedHit.id;
        const oldFillColorIndex = closedHit.fillColorIndex;

        history.execute({
          description: "Fill element",
          execute: () => {
            updateSprite(spriteId, (s) => {
              for (const layer of s.layers) {
                for (const el of layer.elements) {
                  if (el.id === elementId && el.type === "stroke") {
                    el.fillColorIndex = colorIndex;
                  }
                }
              }
            });
          },
          undo: () => {
            updateSprite(spriteId, (s) => {
              for (const layer of s.layers) {
                for (const el of layer.elements) {
                  if (el.id === elementId && el.type === "stroke") {
                    el.fillColorIndex = oldFillColorIndex;
                  }
                }
              }
            });
          },
        });
        return;
      }

      // Nothing hit or hit an open path: set sprite background color
      const oldBgColorIndex = sprite.backgroundColorIndex;
      history.execute({
        description: "Set background color",
        execute: () => {
          updateSprite(spriteId, (s) => {
            s.backgroundColorIndex = colorIndex;
          });
        },
        undo: () => {
          updateSprite(spriteId, (s) => {
            s.backgroundColorIndex = oldBgColorIndex;
          });
        },
      });
    },

    onPointerMove(_e: PointerEvent, canvasPos: Vec2) {
      // Update hover preview
      const hit = hitTestAny(canvasPos);
      hoveredElementId = hit ? hit.id : null;
    },

    onPointerUp(_e: PointerEvent, _canvasPos: Vec2) {
      // No action on pointer up
    },

    render(renderCtx: CanvasRenderingContext2D, _viewport: ViewportState) {
      // Store ctx reference for hit testing
      ctx = renderCtx;

      // Render fill preview highlight on hovered element
      // (handled by the main renderer's selection highlight system)
    },
  };
}
