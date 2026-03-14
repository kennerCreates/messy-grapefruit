// Core types — mirrors Rust data model with camelCase (serde rename_all = "camelCase")

export interface Vec2 {
  x: number;
  y: number;
}

export interface PaletteColor {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface PathVertex {
  id: string;
  pos: Vec2;
  cp1?: Vec2; // cubic bezier handle 1
  cp2?: Vec2; // cubic bezier handle 2
}

export interface StrokeElement {
  id: string;
  name?: string;
  type: "stroke";
  vertices: PathVertex[];
  closed: boolean;
  strokeWidth: number;
  strokeColorIndex: number;
  fillColorIndex: number;
  position: Vec2;
  rotation: number;
  scale: Vec2;
  origin: Vec2;
  visible: boolean;
}

export interface IKTargetElement {
  id: string;
  name?: string;
  type: "ik-target";
  position: Vec2;
  ikChainId: string;
  visible: boolean;
}

export type Element = StrokeElement | IKTargetElement;

export interface Socket {
  parentElementId: string;
  parentVertexId: string;
}

// Layer constraints

export interface SpringParams {
  frequency: number;
  damping: number;
}

export interface GravityParams {
  angle: number;
  strength: number;
}

export interface WindParams {
  strength: number;
  frequency: number;
}

export interface LookAtConstraint {
  targetElementId: string;
  targetVertexId?: string;
  restAngle: number;
  minAngle: number;
  maxAngle: number;
  mix: number;
  smooth?: SpringParams;
}

export interface PhysicsConstraint {
  frequency: number;
  damping: number;
  mix: number;
  gravity?: GravityParams;
  wind?: WindParams;
}

export interface ProceduralModifier {
  property: string;
  waveform: "sine" | "noise";
  amplitude: number;
  frequency: number;
  phase: number;
  blend: "additive" | "multiplicative";
}

export interface LayerConstraints {
  volumePreserve?: boolean;
  lookAt?: LookAtConstraint;
  physics?: PhysicsConstraint;
  procedural?: ProceduralModifier[];
}

export interface Layer {
  id: string;
  name: string;
  visible: boolean;
  locked: boolean;
  elements: Element[];
  socket?: Socket;
  constraints?: LayerConstraints;
}

// Skins

export interface SkinOverride {
  elementId: string;
  strokeColorIndex?: number;
  fillColorIndex?: number;
  strokeWidth?: number;
}

export interface Skin {
  id: string;
  name: string;
  overrides: SkinOverride[];
}

// Animation

export interface EasingCurve {
  preset: string;
  controlPoints: [number, number, number, number];
}

export interface Keyframe {
  id: string;
  time: number;
  value: number;
  easing: EasingCurve;
}

export interface PropertyTrack {
  property: string;
  elementId: string;
  layerId: string;
  keyframes: Keyframe[];
}

export interface AngleConstraint {
  layerId: string;
  min: number;
  max: number;
}

export interface IKChain {
  id: string;
  name: string;
  layerIds: string[];
  targetElementId: string;
  mix: number;
  bendDirection: 1 | -1;
  solver: "two-bone" | "fabrik";
  angleConstraints?: AngleConstraint[];
}

export interface AnimationSequence {
  id: string;
  name: string;
  duration: number;
  looping: boolean;
  tracks: PropertyTrack[];
  ikChains: IKChain[];
}

// Sprite

export interface Sprite {
  id: string;
  name: string;
  formatVersion: string;
  canvasWidth: number;
  canvasHeight: number;
  backgroundColorIndex: number;
  layers: Layer[];
  skins: Skin[];
  animations: AnimationSequence[];
}

// Palette

export interface Palette {
  name: string;
  colors: PaletteColor[];
}

// Project-level types

export type ExportMode = "bone" | "spritesheet";
export type LayoutMode = "row" | "column" | "grid";
export type GridMode = "standard" | "isometric";
export type ThemeMode = "dark" | "light";

export interface ExportSettings {
  mode: ExportMode;
  fps: number;
  layout: LayoutMode;
  trim: boolean;
  padding: number;
}

export interface EditorPreferences {
  theme: ThemeMode;
  gridSize: number;
  gridMode: GridMode;
  showGrid: boolean;
}

export interface ProjectSpriteRef {
  id: string;
  filePath: string;
  position: Vec2;
  rotation: number;
  zOrder: number;
  selectedAnimationId?: string;
  selectedSkinId?: string;
}

export interface Project {
  name: string;
  formatVersion: string;
  exportDir: string;
  palette: Palette;
  sprites: ProjectSpriteRef[];
  exportSettings: ExportSettings;
  editorPreferences: EditorPreferences;
}

// Editor state types (not saved to file)

export type ToolType = "line" | "select" | "fill" | "eraser";

export interface ViewportState {
  panX: number;
  panY: number;
  zoom: number;
}
