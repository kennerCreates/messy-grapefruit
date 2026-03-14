import type { Vec2, StrokeElement, Element, ViewportState } from "../../lib/types";
import { editorStore, setEditorStore } from "../../stores/editor";
import { getActiveSprite, updateSprite } from "../../stores/project";
import { history } from "../../lib/history";
import { generateId, vec2Sub, vec2Add, vec2Distance } from "../../lib/math";
import { hitTestElement } from "../hit-test";
import type { CanvasTool } from "./base";

/** Interaction mode for drag operations */
type DragMode = "none" | "marquee" | "move" | "scale" | "rotate";

/** Handle position identifier for scale handles */
type HandleId = "tl" | "tc" | "tr" | "ml" | "mr" | "bl" | "bc" | "br" | "rot";

interface HandleInfo {
  id: HandleId;
  pos: Vec2;
}

/** Stored reference to the canvas context for hit testing */
let ctx: CanvasRenderingContext2D | null = null;

/** Drag state */
let dragMode: DragMode = "none";
let dragStart: Vec2 = { x: 0, y: 0 };
let dragCurrent: Vec2 = { x: 0, y: 0 };
let dragStartPositions: Map<string, Vec2> = new Map();

/** Scale/rotate state */
let activeHandle: HandleId | null = null;
let scaleStartBBox: { x: number; y: number; w: number; h: number } | null = null;
let scaleStartScales: Map<string, Vec2> = new Map();
let scaleStartPositions: Map<string, Vec2> = new Map();
let rotateStartAngle: number = 0;
let rotateStartRotations: Map<string, number> = new Map();

/**
 * Get all elements from visible, unlocked layers.
 */
function getAllEditableElements(): { element: Element; layerId: string; layerIndex: number }[] {
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
 * Hit test all editable elements at a position.
 * Returns the topmost hit element (last layer, last element = on top).
 */
function hitTestAll(pos: Vec2): { element: Element; layerId: string; layerIndex: number } | null {
  if (!ctx) return null;
  const allElements = getAllEditableElements();
  // Iterate in reverse for top-most-first hit testing
  for (let i = allElements.length - 1; i >= 0; i--) {
    const entry = allElements[i];
    if (entry.element.type === "stroke") {
      if (hitTestElement(pos, entry.element as StrokeElement, ctx)) {
        return entry;
      }
    }
  }
  return null;
}

/**
 * Compute bounding box of a single StrokeElement in canvas space.
 */
function elementBBox(el: StrokeElement): { x: number; y: number; w: number; h: number } {
  if (el.vertices.length === 0) return { x: 0, y: 0, w: 0, h: 0 };
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const v of el.vertices) {
    // Apply element transform to vertex position
    const px = el.position.x + (v.pos.x - el.origin.x) * el.scale.x;
    const py = el.position.y + (v.pos.y - el.origin.y) * el.scale.y;
    minX = Math.min(minX, px);
    minY = Math.min(minY, py);
    maxX = Math.max(maxX, px);
    maxY = Math.max(maxY, py);
  }
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

/**
 * Check if a rectangle (x,y,w,h) intersects with the marquee (defined by two corners).
 */
function rectsIntersect(
  a: { x: number; y: number; w: number; h: number },
  b: { x: number; y: number; w: number; h: number },
): boolean {
  return !(
    a.x + a.w < b.x ||
    b.x + b.w < a.x ||
    a.y + a.h < b.y ||
    b.y + b.h < a.y
  );
}

/**
 * Get the marquee rectangle from dragStart/dragCurrent as normalized {x,y,w,h}.
 */
function getMarqueeRect(): { x: number; y: number; w: number; h: number } {
  const x = Math.min(dragStart.x, dragCurrent.x);
  const y = Math.min(dragStart.y, dragCurrent.y);
  const w = Math.abs(dragCurrent.x - dragStart.x);
  const h = Math.abs(dragCurrent.y - dragStart.y);
  return { x, y, w, h };
}

/**
 * Get selected elements from the store.
 */
function getSelectedElements(): StrokeElement[] {
  const sprite = getActiveSprite();
  if (!sprite) return [];
  const selectedIds = editorStore.selectedElementIds;
  const results: StrokeElement[] = [];
  for (const layer of sprite.layers) {
    for (const el of layer.elements) {
      if (el.type === "stroke" && selectedIds.includes(el.id)) {
        results.push(el);
      }
    }
  }
  return results;
}

/**
 * Get the combined bounding box of all selected elements.
 */
function getSelectionBBox(): { x: number; y: number; w: number; h: number } | null {
  const elements = getSelectedElements();
  if (elements.length === 0) return null;
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const el of elements) {
    const bb = elementBBox(el);
    minX = Math.min(minX, bb.x);
    minY = Math.min(minY, bb.y);
    maxX = Math.max(maxX, bb.x + bb.w);
    maxY = Math.max(maxY, bb.y + bb.h);
  }
  if (minX > maxX) return null;
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

/**
 * Compute the 8 scale handles and 1 rotation handle for a bounding box.
 */
function getHandles(bbox: { x: number; y: number; w: number; h: number }): HandleInfo[] {
  const { x, y, w, h } = bbox;
  const mx = x + w / 2;
  const my = y + h / 2;
  return [
    { id: "tl", pos: { x, y } },
    { id: "tc", pos: { x: mx, y } },
    { id: "tr", pos: { x: x + w, y } },
    { id: "ml", pos: { x, y: my } },
    { id: "mr", pos: { x: x + w, y: my } },
    { id: "bl", pos: { x, y: y + h } },
    { id: "bc", pos: { x: mx, y: y + h } },
    { id: "br", pos: { x: x + w, y: y + h } },
    { id: "rot", pos: { x: mx, y: y - 20 } }, // rotation handle above top-center (offset in canvas space, adjusted in render)
  ];
}

/**
 * Check if a canvas position is near any handle. Returns the handle ID or null.
 */
function hitTestHandle(pos: Vec2, viewport: ViewportState): HandleId | null {
  const bbox = getSelectionBBox();
  if (!bbox) return null;
  const handles = getHandles(bbox);
  const threshold = 8 / viewport.zoom; // screen-space threshold
  for (const h of handles) {
    // For rotation handle, adjust y offset by zoom
    const handlePos = h.id === "rot"
      ? { x: h.pos.x, y: bbox.y - 20 / viewport.zoom }
      : h.pos;
    if (vec2Distance(pos, handlePos) <= threshold) {
      return h.id;
    }
  }
  return null;
}

export function createSelectTool(): CanvasTool {
  return {
    name: "select",
    cursor: "default",

    onPointerDown(e: PointerEvent, canvasPos: Vec2) {
      const selectedIds = editorStore.selectedElementIds;

      // Check for scale/rotate handle hit first (only when exactly 1 element selected)
      if (selectedIds.length === 1) {
        const handleHit = hitTestHandle(canvasPos, editorStore.viewport);
        if (handleHit) {
          activeHandle = handleHit;
          dragStart = canvasPos;
          dragCurrent = canvasPos;

          if (handleHit === "rot") {
            dragMode = "rotate";
            const bbox = getSelectionBBox()!;
            const center = { x: bbox.x + bbox.w / 2, y: bbox.y + bbox.h / 2 };
            rotateStartAngle = Math.atan2(canvasPos.y - center.y, canvasPos.x - center.x);
            rotateStartRotations = new Map();
            for (const el of getSelectedElements()) {
              rotateStartRotations.set(el.id, el.rotation);
            }
          } else {
            dragMode = "scale";
            scaleStartBBox = getSelectionBBox();
            scaleStartScales = new Map();
            scaleStartPositions = new Map();
            for (const el of getSelectedElements()) {
              scaleStartScales.set(el.id, { ...el.scale });
              scaleStartPositions.set(el.id, { ...el.position });
            }
          }
          return;
        }
      }

      // Hit test elements
      const hit = hitTestAll(canvasPos);

      if (hit) {
        const isAlreadySelected = selectedIds.includes(hit.element.id);

        if (e.shiftKey) {
          // Toggle selection
          if (isAlreadySelected) {
            setEditorStore(
              "selectedElementIds",
              selectedIds.filter((id) => id !== hit.element.id),
            );
          } else {
            setEditorStore("selectedElementIds", [...selectedIds, hit.element.id]);
          }
        } else if (isAlreadySelected) {
          // Start move drag
          dragMode = "move";
          dragStart = canvasPos;
          dragCurrent = canvasPos;
          dragStartPositions = new Map();
          for (const el of getSelectedElements()) {
            dragStartPositions.set(el.id, { ...el.position });
          }
        } else {
          // Select just this element
          setEditorStore("selectedElementIds", [hit.element.id]);
          // Start move drag
          dragMode = "move";
          dragStart = canvasPos;
          dragCurrent = canvasPos;
          dragStartPositions = new Map();
          // Re-query selected after updating store
          const sprite = getActiveSprite();
          if (sprite) {
            for (const layer of sprite.layers) {
              for (const el of layer.elements) {
                if (el.type === "stroke" && el.id === hit.element.id) {
                  dragStartPositions.set(el.id, { ...el.position });
                }
              }
            }
          }
        }
      } else {
        if (!e.shiftKey) {
          // Deselect all
          setEditorStore("selectedElementIds", []);
        }
        // Start marquee selection
        dragMode = "marquee";
        dragStart = canvasPos;
        dragCurrent = canvasPos;
      }
    },

    onPointerMove(_e: PointerEvent, canvasPos: Vec2) {
      dragCurrent = canvasPos;

      if (dragMode === "move") {
        // Live preview: move selected elements
        const delta = vec2Sub(canvasPos, dragStart);
        const snappedDelta = {
          x: Math.round(delta.x / editorStore.gridSize) * editorStore.gridSize,
          y: Math.round(delta.y / editorStore.gridSize) * editorStore.gridSize,
        };
        const sprite = getActiveSprite();
        if (!sprite) return;
        const spriteId = sprite.id;
        updateSprite(spriteId, (s) => {
          for (const layer of s.layers) {
            for (const el of layer.elements) {
              if (el.type === "stroke" && dragStartPositions.has(el.id)) {
                const startPos = dragStartPositions.get(el.id)!;
                el.position = vec2Add(startPos, snappedDelta);
              }
            }
          }
        });
      } else if (dragMode === "scale" && scaleStartBBox && activeHandle) {
        const sprite = getActiveSprite();
        if (!sprite) return;
        const spriteId = sprite.id;
        const bbox = scaleStartBBox;

        // Calculate scale factors based on handle direction
        let sx = 1, sy = 1;
        if (bbox.w > 0) {
          if (activeHandle === "tl" || activeHandle === "ml" || activeHandle === "bl") {
            sx = (bbox.x + bbox.w - canvasPos.x) / bbox.w;
          } else if (activeHandle === "tr" || activeHandle === "mr" || activeHandle === "br") {
            sx = (canvasPos.x - bbox.x) / bbox.w;
          }
        }
        if (bbox.h > 0) {
          if (activeHandle === "tl" || activeHandle === "tc" || activeHandle === "tr") {
            sy = (bbox.y + bbox.h - canvasPos.y) / bbox.h;
          } else if (activeHandle === "bl" || activeHandle === "bc" || activeHandle === "br") {
            sy = (canvasPos.y - bbox.y) / bbox.h;
          }
        }

        // Only scale in the relevant axes
        if (activeHandle === "ml" || activeHandle === "mr") sy = 1;
        if (activeHandle === "tc" || activeHandle === "bc") sx = 1;

        // Prevent zero/negative scale
        sx = Math.max(0.01, Math.abs(sx)) * Math.sign(sx || 1);
        sy = Math.max(0.01, Math.abs(sy)) * Math.sign(sy || 1);

        updateSprite(spriteId, (s) => {
          for (const layer of s.layers) {
            for (const el of layer.elements) {
              if (el.type === "stroke" && scaleStartScales.has(el.id)) {
                const startScale = scaleStartScales.get(el.id)!;
                el.scale = {
                  x: startScale.x * sx,
                  y: startScale.y * sy,
                };
              }
            }
          }
        });
      } else if (dragMode === "rotate") {
        const sprite = getActiveSprite();
        if (!sprite) return;
        const spriteId = sprite.id;
        const bbox = getSelectionBBox();
        if (!bbox) return;
        const center = { x: bbox.x + bbox.w / 2, y: bbox.y + bbox.h / 2 };
        const currentAngle = Math.atan2(canvasPos.y - center.y, canvasPos.x - center.x);
        const angleDelta = currentAngle - rotateStartAngle;

        updateSprite(spriteId, (s) => {
          for (const layer of s.layers) {
            for (const el of layer.elements) {
              if (el.type === "stroke" && rotateStartRotations.has(el.id)) {
                const startRot = rotateStartRotations.get(el.id)!;
                el.rotation = startRot + angleDelta;
              }
            }
          }
        });
      }
    },

    onPointerUp(_e: PointerEvent, canvasPos: Vec2) {
      if (dragMode === "marquee") {
        // Select all elements whose bounding box intersects the marquee
        const marquee = getMarqueeRect();
        if (marquee.w > 2 || marquee.h > 2) {
          const allElements = getAllEditableElements();
          const hitIds: string[] = [];
          for (const entry of allElements) {
            if (entry.element.type === "stroke") {
              const bb = elementBBox(entry.element as StrokeElement);
              if (rectsIntersect(bb, marquee)) {
                hitIds.push(entry.element.id);
              }
            }
          }
          if (_e.shiftKey) {
            // Additive marquee
            const existing = new Set(editorStore.selectedElementIds);
            for (const id of hitIds) existing.add(id);
            setEditorStore("selectedElementIds", [...existing]);
          } else {
            setEditorStore("selectedElementIds", hitIds);
          }
        }
      } else if (dragMode === "move") {
        // Commit the move via history
        const delta = vec2Sub(canvasPos, dragStart);
        const snappedDelta = {
          x: Math.round(delta.x / editorStore.gridSize) * editorStore.gridSize,
          y: Math.round(delta.y / editorStore.gridSize) * editorStore.gridSize,
        };
        if (snappedDelta.x !== 0 || snappedDelta.y !== 0) {
          const sprite = getActiveSprite();
          if (sprite) {
            const spriteId = sprite.id;
            const movedIds = [...dragStartPositions.keys()];
            const originalPositions = new Map(dragStartPositions);
            // Already moved live; just record the history command for undo
            history.execute({
              description: "Move elements",
              execute: () => {
                updateSprite(spriteId, (s) => {
                  for (const layer of s.layers) {
                    for (const el of layer.elements) {
                      if (movedIds.includes(el.id) && originalPositions.has(el.id)) {
                        el.position = vec2Add(originalPositions.get(el.id)!, snappedDelta);
                      }
                    }
                  }
                });
              },
              undo: () => {
                updateSprite(spriteId, (s) => {
                  for (const layer of s.layers) {
                    for (const el of layer.elements) {
                      if (movedIds.includes(el.id) && originalPositions.has(el.id)) {
                        el.position = { ...originalPositions.get(el.id)! };
                      }
                    }
                  }
                });
              },
            });
          }
        }
      } else if (dragMode === "scale") {
        // Commit scale via history
        const sprite = getActiveSprite();
        if (sprite) {
          const spriteId = sprite.id;
          const originalScales = new Map(scaleStartScales);
          const originalPositions = new Map(scaleStartPositions);
          // Capture current scales
          const newScales = new Map<string, Vec2>();
          for (const el of getSelectedElements()) {
            newScales.set(el.id, { ...el.scale });
          }
          history.execute({
            description: "Scale elements",
            execute: () => {
              updateSprite(spriteId, (s) => {
                for (const layer of s.layers) {
                  for (const el of layer.elements) {
                    if (el.type === "stroke" && newScales.has(el.id)) {
                      el.scale = { ...newScales.get(el.id)! };
                    }
                  }
                }
              });
            },
            undo: () => {
              updateSprite(spriteId, (s) => {
                for (const layer of s.layers) {
                  for (const el of layer.elements) {
                    if (el.type === "stroke" && originalScales.has(el.id)) {
                      el.scale = { ...originalScales.get(el.id)! };
                      el.position = { ...originalPositions.get(el.id)! };
                    }
                  }
                }
              });
            },
          });
        }
      } else if (dragMode === "rotate") {
        // Commit rotation via history
        const sprite = getActiveSprite();
        if (sprite) {
          const spriteId = sprite.id;
          const originalRotations = new Map(rotateStartRotations);
          const newRotations = new Map<string, number>();
          for (const el of getSelectedElements()) {
            newRotations.set(el.id, el.rotation);
          }
          history.execute({
            description: "Rotate elements",
            execute: () => {
              updateSprite(spriteId, (s) => {
                for (const layer of s.layers) {
                  for (const el of layer.elements) {
                    if (el.type === "stroke" && newRotations.has(el.id)) {
                      el.rotation = newRotations.get(el.id)!;
                    }
                  }
                }
              });
            },
            undo: () => {
              updateSprite(spriteId, (s) => {
                for (const layer of s.layers) {
                  for (const el of layer.elements) {
                    if (el.type === "stroke" && originalRotations.has(el.id)) {
                      el.rotation = originalRotations.get(el.id)!;
                    }
                  }
                }
              });
            },
          });
        }
      }

      // Reset drag state
      dragMode = "none";
      activeHandle = null;
      scaleStartBBox = null;
      scaleStartScales.clear();
      scaleStartPositions.clear();
      rotateStartRotations.clear();
      dragStartPositions.clear();
    },

    onKeyDown(e: KeyboardEvent) {
      // Delete selected elements
      if (e.key === "Delete" || e.key === "Backspace") {
        const selectedIds = editorStore.selectedElementIds;
        if (selectedIds.length === 0) return;
        e.preventDefault();

        const sprite = getActiveSprite();
        if (!sprite) return;
        const spriteId = sprite.id;

        // Snapshot the elements being deleted for undo
        const deletedElements: { layerIndex: number; element: Element }[] = [];
        for (let li = 0; li < sprite.layers.length; li++) {
          const layer = sprite.layers[li];
          for (const el of layer.elements) {
            if (selectedIds.includes(el.id)) {
              deletedElements.push({
                layerIndex: li,
                element: JSON.parse(JSON.stringify(el)),
              });
            }
          }
        }

        history.execute({
          description: "Delete elements",
          execute: () => {
            updateSprite(spriteId, (s) => {
              for (const layer of s.layers) {
                layer.elements = layer.elements.filter(
                  (el) => !selectedIds.includes(el.id),
                );
              }
            });
            setEditorStore("selectedElementIds", []);
          },
          undo: () => {
            updateSprite(spriteId, (s) => {
              for (const item of deletedElements) {
                if (item.layerIndex < s.layers.length) {
                  s.layers[item.layerIndex].elements.push(
                    JSON.parse(JSON.stringify(item.element)),
                  );
                }
              }
            });
            setEditorStore("selectedElementIds", selectedIds);
          },
        });
        return;
      }

      // Ctrl+A: Select all
      if (e.ctrlKey && (e.key === "a" || e.key === "A")) {
        e.preventDefault();
        const allElements = getAllEditableElements();
        const allIds = allElements.map((e) => e.element.id);
        setEditorStore("selectedElementIds", allIds);
        return;
      }

      // Ctrl+C: Copy
      if (e.ctrlKey && (e.key === "c" || e.key === "C")) {
        e.preventDefault();
        const elements = getSelectedElements();
        if (elements.length === 0) return;
        const json = JSON.stringify(elements);
        navigator.clipboard.writeText(json).catch(() => {
          // Silently fail if clipboard is not available
        });
        return;
      }

      // Ctrl+V: Paste
      if (e.ctrlKey && (e.key === "v" || e.key === "V")) {
        e.preventDefault();
        navigator.clipboard
          .readText()
          .then((text) => {
            let elements: StrokeElement[];
            try {
              elements = JSON.parse(text);
            } catch {
              return;
            }
            if (!Array.isArray(elements) || elements.length === 0) return;

            const sprite = getActiveSprite();
            if (!sprite) return;
            const spriteId = sprite.id;

            // Create new layer with pasted elements
            const newLayerId = generateId();
            const newElements: StrokeElement[] = elements.map((el) => ({
              ...el,
              id: generateId(),
              position: vec2Add(el.position, { x: 10, y: 10 }),
              vertices: el.vertices.map((v) => ({
                ...v,
                id: generateId(),
              })),
            }));

            const newIds = newElements.map((el) => el.id);

            history.execute({
              description: "Paste elements",
              execute: () => {
                updateSprite(spriteId, (s) => {
                  s.layers.push({
                    id: newLayerId,
                    name: "Pasted",
                    visible: true,
                    locked: false,
                    elements: JSON.parse(JSON.stringify(newElements)),
                  });
                });
                setEditorStore("selectedElementIds", newIds);
              },
              undo: () => {
                updateSprite(spriteId, (s) => {
                  s.layers = s.layers.filter((l) => l.id !== newLayerId);
                });
                setEditorStore("selectedElementIds", []);
              },
            });
          })
          .catch(() => {
            // Clipboard not available
          });
        return;
      }
    },

    render(renderCtx: CanvasRenderingContext2D, viewport: ViewportState) {
      // Store ctx reference for hit testing
      ctx = renderCtx;

      // Draw marquee selection rectangle
      if (dragMode === "marquee") {
        const rect = getMarqueeRect();
        renderCtx.save();
        renderCtx.fillStyle = "rgba(0, 120, 255, 0.15)";
        renderCtx.strokeStyle = "rgba(0, 120, 255, 0.8)";
        renderCtx.lineWidth = 1 / viewport.zoom;
        renderCtx.fillRect(rect.x, rect.y, rect.w, rect.h);
        renderCtx.strokeRect(rect.x, rect.y, rect.w, rect.h);
        renderCtx.restore();
      }

      // Draw scale/rotate handles when exactly one element is selected
      if (editorStore.selectedElementIds.length === 1) {
        const bbox = getSelectionBBox();
        if (bbox) {
          renderCtx.save();

          const handleSize = 6 / viewport.zoom;
          const rotHandleOffset = 20 / viewport.zoom;

          // Draw bounding box
          renderCtx.strokeStyle = "rgba(0, 170, 255, 0.6)";
          renderCtx.lineWidth = 1 / viewport.zoom;
          renderCtx.setLineDash([4 / viewport.zoom, 4 / viewport.zoom]);
          renderCtx.strokeRect(bbox.x, bbox.y, bbox.w, bbox.h);
          renderCtx.setLineDash([]);

          // Scale handles
          const corners: { id: HandleId; x: number; y: number }[] = [
            { id: "tl", x: bbox.x, y: bbox.y },
            { id: "tc", x: bbox.x + bbox.w / 2, y: bbox.y },
            { id: "tr", x: bbox.x + bbox.w, y: bbox.y },
            { id: "ml", x: bbox.x, y: bbox.y + bbox.h / 2 },
            { id: "mr", x: bbox.x + bbox.w, y: bbox.y + bbox.h / 2 },
            { id: "bl", x: bbox.x, y: bbox.y + bbox.h },
            { id: "bc", x: bbox.x + bbox.w / 2, y: bbox.y + bbox.h },
            { id: "br", x: bbox.x + bbox.w, y: bbox.y + bbox.h },
          ];

          for (const corner of corners) {
            renderCtx.fillStyle = "#ffffff";
            renderCtx.strokeStyle = "#00aaff";
            renderCtx.lineWidth = 1.5 / viewport.zoom;
            renderCtx.fillRect(
              corner.x - handleSize / 2,
              corner.y - handleSize / 2,
              handleSize,
              handleSize,
            );
            renderCtx.strokeRect(
              corner.x - handleSize / 2,
              corner.y - handleSize / 2,
              handleSize,
              handleSize,
            );
          }

          // Rotation handle (circle above top-center)
          const rotX = bbox.x + bbox.w / 2;
          const rotY = bbox.y - rotHandleOffset;

          // Line from top-center to rotation handle
          renderCtx.beginPath();
          renderCtx.moveTo(rotX, bbox.y);
          renderCtx.lineTo(rotX, rotY);
          renderCtx.strokeStyle = "rgba(0, 170, 255, 0.6)";
          renderCtx.lineWidth = 1 / viewport.zoom;
          renderCtx.stroke();

          // Rotation circle
          renderCtx.beginPath();
          renderCtx.arc(rotX, rotY, handleSize / 2, 0, Math.PI * 2);
          renderCtx.fillStyle = "#ffffff";
          renderCtx.fill();
          renderCtx.strokeStyle = "#00aaff";
          renderCtx.lineWidth = 1.5 / viewport.zoom;
          renderCtx.stroke();

          renderCtx.restore();
        }
      }
    },
  };
}
