import { createStore, produce } from "solid-js/store";
import type { ToolType, ViewportState, GridMode, PathVertex, Vec2 } from "../lib/types";
import {
  MIN_ZOOM,
  MAX_ZOOM,
  ZOOM_SPEED,
  DEFAULT_STROKE_WIDTH,
  DEFAULT_GRID_SIZE,
} from "../lib/constants";
import { clamp } from "../lib/math";

export interface EditorState {
  activeTool: ToolType;
  activeColorIndex: number;
  activeLayerId: string | null;
  strokeWidth: number;
  curveMode: boolean;
  viewport: ViewportState;
  selectedElementIds: string[];
  selectedVertexIds: string[];
  hoveredElementId: string | null;
  hoveredVertexId: string | null;
  isDrawing: boolean;
  currentStrokeVertices: PathVertex[];
  showGrid: boolean;
  gridMode: GridMode;
  gridSize: number;
}

const initialState: EditorState = {
  activeTool: "line",
  activeColorIndex: 1, // index 0 is typically "none" / transparent
  activeLayerId: null,
  strokeWidth: DEFAULT_STROKE_WIDTH,
  curveMode: true,
  viewport: { panX: 0, panY: 0, zoom: 1 },
  selectedElementIds: [],
  selectedVertexIds: [],
  hoveredElementId: null,
  hoveredVertexId: null,
  isDrawing: false,
  currentStrokeVertices: [],
  showGrid: true,
  gridMode: "standard",
  gridSize: DEFAULT_GRID_SIZE,
};

export const [editorStore, setEditorStore] = createStore<EditorState>(initialState);

// --- Tool ---

export function setActiveTool(tool: ToolType) {
  setEditorStore("activeTool", tool);
}

// --- Active layer ---

export function setActiveLayerId(layerId: string | null) {
  setEditorStore("activeLayerId", layerId);
}

// --- Color & stroke ---

export function setActiveColor(index: number) {
  setEditorStore("activeColorIndex", index);
}

export function setStrokeWidth(w: number) {
  setEditorStore("strokeWidth", Math.max(0.5, w));
}

export function toggleCurveMode() {
  setEditorStore("curveMode", (prev) => !prev);
}

// --- Viewport ---

export function setViewport(v: Partial<ViewportState>) {
  setEditorStore("viewport", (prev) => ({ ...prev, ...v }));
}

/**
 * Zoom centered on a screen-space point.
 * delta > 0 zooms in, delta < 0 zooms out.
 */
export function zoomAt(screenX: number, screenY: number, delta: number) {
  setEditorStore(
    produce((state) => {
      const oldZoom = state.viewport.zoom;
      const newZoom = clamp(oldZoom * Math.exp(-delta * ZOOM_SPEED), MIN_ZOOM, MAX_ZOOM);
      const ratio = newZoom / oldZoom;

      // Adjust pan so the point under the cursor stays fixed
      state.viewport.panX = screenX - (screenX - state.viewport.panX) * ratio;
      state.viewport.panY = screenY - (screenY - state.viewport.panY) * ratio;
      state.viewport.zoom = newZoom;
    }),
  );
}

export function panBy(dx: number, dy: number) {
  setEditorStore(
    produce((state) => {
      state.viewport.panX += dx;
      state.viewport.panY += dy;
    }),
  );
}

// --- Selection ---

export function selectElement(id: string, additive = false) {
  setEditorStore(
    produce((state) => {
      if (additive) {
        const idx = state.selectedElementIds.indexOf(id);
        if (idx >= 0) {
          state.selectedElementIds.splice(idx, 1);
        } else {
          state.selectedElementIds.push(id);
        }
      } else {
        state.selectedElementIds = [id];
      }
    }),
  );
}

export function selectVertex(id: string, additive = false) {
  setEditorStore(
    produce((state) => {
      if (additive) {
        const idx = state.selectedVertexIds.indexOf(id);
        if (idx >= 0) {
          state.selectedVertexIds.splice(idx, 1);
        } else {
          state.selectedVertexIds.push(id);
        }
      } else {
        state.selectedVertexIds = [id];
      }
    }),
  );
}

export function deselectAll() {
  setEditorStore(
    produce((state) => {
      state.selectedElementIds = [];
      state.selectedVertexIds = [];
    }),
  );
}

export function setHoveredElement(id: string | null) {
  setEditorStore("hoveredElementId", id);
}

export function setHoveredVertex(id: string | null) {
  setEditorStore("hoveredVertexId", id);
}

// --- Drawing state ---

export function setIsDrawing(drawing: boolean) {
  setEditorStore("isDrawing", drawing);
}

export function setCurrentStrokeVertices(vertices: PathVertex[]) {
  setEditorStore("currentStrokeVertices", vertices);
}

export function addCurrentStrokeVertex(vertex: PathVertex) {
  setEditorStore(
    produce((state) => {
      state.currentStrokeVertices.push(vertex);
    }),
  );
}

export function clearCurrentStroke() {
  setEditorStore("currentStrokeVertices", []);
  setEditorStore("isDrawing", false);
}

// --- Grid ---

export function setShowGrid(show: boolean) {
  setEditorStore("showGrid", show);
}

export function setGridMode(mode: GridMode) {
  setEditorStore("gridMode", mode);
}

export function setGridSize(size: number) {
  setEditorStore("gridSize", Math.max(1, size));
}

// --- Coordinate conversion ---

/**
 * Convert screen (pixel) coordinates to canvas (world) coordinates.
 */
export function screenToCanvas(screenX: number, screenY: number): Vec2 {
  const { panX, panY, zoom } = editorStore.viewport;
  return {
    x: (screenX - panX) / zoom,
    y: (screenY - panY) / zoom,
  };
}

/**
 * Convert canvas (world) coordinates to screen (pixel) coordinates.
 */
export function canvasToScreen(canvasX: number, canvasY: number): Vec2 {
  const { panX, panY, zoom } = editorStore.viewport;
  return {
    x: canvasX * zoom + panX,
    y: canvasY * zoom + panY,
  };
}
