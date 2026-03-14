import { produce } from "solid-js/store";
import type { PaletteColor } from "../lib/types";
import { MAX_PALETTE_COLORS } from "../lib/constants";
import { projectStore, setProjectStore } from "./project";
import { editorStore } from "./editor";

// --- Palette accessors ---

/**
 * Reactive accessor for the project palette colors.
 */
export function paletteColors(): PaletteColor[] {
  return projectStore.project?.palette.colors ?? [];
}

/**
 * Reactive accessor: the currently active color based on editor's activeColorIndex.
 */
export function activeColor(): PaletteColor | null {
  const colors = paletteColors();
  const index = editorStore.activeColorIndex;
  if (index < 0 || index >= colors.length) return null;
  return colors[index];
}

/**
 * Get a palette color as a CSS-compatible rgba string.
 */
export function colorToCSS(color: PaletteColor): string {
  return `rgba(${color.r}, ${color.g}, ${color.b}, ${color.a / 255})`;
}

/**
 * Get a palette color by index as a CSS string, or return a fallback.
 */
export function paletteColorCSS(index: number, fallback = "transparent"): string {
  const colors = paletteColors();
  if (index < 0 || index >= colors.length) return fallback;
  return colorToCSS(colors[index]);
}

// --- Palette mutations ---

export function addColor(color: PaletteColor) {
  setProjectStore(
    produce((state) => {
      if (!state.project) return;
      if (state.project.palette.colors.length >= MAX_PALETTE_COLORS) return;
      state.project.palette.colors.push(color);
      state.dirty = true;
    }),
  );
}

export function removeColor(index: number) {
  setProjectStore(
    produce((state) => {
      if (!state.project) return;
      const colors = state.project.palette.colors;
      if (index < 0 || index >= colors.length) return;
      colors.splice(index, 1);
      state.dirty = true;
    }),
  );
}

export function updateColor(index: number, color: PaletteColor) {
  setProjectStore(
    produce((state) => {
      if (!state.project) return;
      const colors = state.project.palette.colors;
      if (index < 0 || index >= colors.length) return;
      colors[index] = color;
      state.dirty = true;
    }),
  );
}

/**
 * Replace the entire palette with a new set of colors and optionally update the palette name.
 */
export function replaceAllColors(colors: PaletteColor[], name?: string) {
  setProjectStore(
    produce((state) => {
      if (!state.project) return;
      state.project.palette.colors = colors;
      if (name) state.project.palette.name = name;
      state.dirty = true;
    }),
  );
}
