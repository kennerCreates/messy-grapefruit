export const FORMAT_VERSION = "1.0";
export const MAX_PALETTE_COLORS = 256;
export const AUTOSAVE_DELAY_MS = 3000;
export const DEFAULT_CANVAS_WIDTH = 256;
export const DEFAULT_CANVAS_HEIGHT = 256;
export const DEFAULT_STROKE_WIDTH = 2;
export const DEFAULT_GRID_SIZE = 16;
export const MIN_ZOOM = 0.1;
export const MAX_ZOOM = 32;
export const ZOOM_SPEED = 0.001;
export const MERGE_THRESHOLD = 0.5; // grid units
export const GRID_DOT_RADIUS = 1.5;
export const GRID_DENSITY_SCREEN_PX = 32; // target screen-space spacing

// Adaptive grid thresholds: zoom level -> grid subdivision
export const GRID_ZOOM_THRESHOLDS = [
  { zoom: 0.25, gridDiv: 8 },
  { zoom: 0.5, gridDiv: 4 },
  { zoom: 1, gridDiv: 2 },
  { zoom: 2, gridDiv: 1 },
  { zoom: 4, gridDiv: 0.5 },
  { zoom: 8, gridDiv: 0.25 },
];

// Theme colors
export const THEMES = {
  dark: {
    bg: "#292831",
    panels: "#333f58",
    accent: "#4a7a96",
    secondary: "#ee8695",
    text: "#fbbbad",
  },
  light: {
    bg: "#ffecd6",
    panels: "#ffb873",
    accent: "#cb765c",
    secondary: "#7a4a5a",
    text: "#25213e",
  },
} as const;
