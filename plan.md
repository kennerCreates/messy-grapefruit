# SVG Sprite Drawing & Animation Tool

## Context

Building a Tauri desktop app (Rust backend + SolidJS frontend) for creating animated SVG sprites for a 2D isometric Bevy game. The tool draws vector art using lines/curves with an indexed color palette, animates via keyframes with editable easing curves, and auto-exports PNG spritesheets + RON metadata that Bevy hot-reloads.

---

## Architecture

- **Tauri v2** — Rust backend for file I/O, SVG generation, PNG rasterization (`resvg`), spritesheet packing, file watching (`notify`), RON export
- **SolidJS + TypeScript** — Frontend with HTML Canvas 2D for drawing, SolidJS stores for reactive state
- **HTML Canvas 2D** — Drawing surface. Native `lineCap="round"`, `lineJoin="round"`, `Path2D`, `bezierCurveTo`, `isPointInStroke` for hit-testing
- **JSON file format** — `.sprite` (per sprite) and `.spriteproj` (project overview). Human-readable, diffable, trivial serde

### Why these choices
- **SolidJS over React**: Fine-grained reactivity without VDOM. Deep store paths like `setStore("layers", i, "elements", j, "vertices", k, "x", val)` update only the one binding that reads it. Critical for a drawing app with many elements.
- **Canvas 2D over SVG DOM**: Imperative rendering is faster for animation preview and avoids framework/DOM conflicts. Built-in curve and stroke support.
- **JSON over binary**: Sprite files are small (vector data). Human-readable, version-controllable, diffable.

---

## Data Model

### Core Types (TypeScript — Rust mirrors these with serde)

```
Project (.spriteproj file)
├── name, version, exportDir
├── palette: Palette { name, colors: PaletteColor[] }  // index 0 = transparent, shared by ALL sprites
├── sprites: ProjectSpriteRef[] (id, filePath, position, rotation, zOrder)  // position/rotation/zOrder are for dashboard layout only, not game-spatial
└── editorPreferences (theme, gridSize, gridMode, showGrid)

Sprite (.sprite file)  // canvasWidth/canvasHeight = export pixel dimensions (1:1, no scale factor)
├── id, name, canvasWidth, canvasHeight, backgroundColorIndex (default 0/transparent)
├── layers: Layer[]
│   └── Layer { id, name, visible, locked, elements: Element[] }
│       └── StrokeElement { id, vertices: PathVertex[], closed, strokeWidth,
│           strokeColorIndex, fillColorIndex, position, rotation, scale, origin: Vec2 }
│           └── PathVertex { id, pos: Vec2, cp1?: Vec2, cp2?: Vec2 }
│               (cp1/cp2 = cubic bezier handles; absent = straight line)
│               (origin = user-defined pivot point for rotation/scale, snaps to grid)
└── animations: AnimationSequence[]
    └── AnimationSequence { id, name, duration, looping, tracks: PropertyTrack[] }
        └── PropertyTrack { property, elementId, layerId, keyframes: Keyframe[] }
            └── Keyframe { id, time, value, easing: EasingCurve }
                └── EasingCurve { preset, controlPoints: [x1,y1,x2,y2] }
```

**Animatable properties**: `position.x`, `position.y`, `rotation`, `scale.x`, `scale.y`, `strokeColorIndex`, `fillColorIndex`, `vertex.{id}.x`, `vertex.{id}.y`, `visible`

*Vertex animation uses stable vertex IDs (not positional indices) so tracks survive vertex insertion/deletion.*

**Visibility & drawing on non-zero frames**: Elements have a `visible` property (animatable, hold-previous interpolation). Drawing a new element while the playhead is at a non-zero time auto-creates a visibility track: hidden before the current time, visible from the current time onward.

---

## Key Behaviors

### Viewport controls
- **Zoom**: Scroll wheel (centered on cursor)
- **Pan**: Middle-click drag

### Auto-merge vertices
When a vertex is placed at the same grid position as an existing vertex on the same layer, the elements **fuse into a single StrokeElement** with a combined vertex list. This enables connected paths and joined shapes. The merge is based on exact grid-snapped coordinates. Cross-element merges fuse the two elements; same-element merges close the path. Undo captures the pre-merge state of both elements so the merge can be fully reversed.

### Curve handles & straight/curve toggle
- **Auto-curve is the default**: When placing vertices, control points are auto-generated using Catmull-Rom → cubic bezier conversion. Endpoints use duplicated-endpoint phantom points (zero curvature / straight tangent at path ends)
- **Editable handles**: Selected curved vertices display draggable cp1/cp2 control point handles on the canvas. Dragging a handle updates the bezier curve in real-time
- **Straight/curve hotkey toggle**: A single hotkey (e.g., `C`) toggles the line tool between curve mode and straight mode while drawing. Indicated visually in toolbar/status bar. Can also toggle per-vertex after placement by selecting a vertex and pressing the hotkey

### Adaptive grid
Grid density changes automatically with zoom level:
- Each zoom threshold maps to a power-of-2 grid size
- Zoom in → finer grid (smaller spacing), zoom out → coarser grid (larger spacing)
- Grid dots stay at a visually consistent screen-space density regardless of zoom
- Snapping follows the currently visible grid level

### Indexed color palette (per-project)
- The palette lives on the **Project**, not on individual sprites — all sprites in a project share the same palette
- Elements store palette index, not color values
- Changing a palette color instantly updates all elements across all sprites referencing that index (renderer looks up color at render time)
- Index 0 is always transparent/none
- Color index animation uses **hold-previous** step interpolation (value holds at current keyframe until next keyframe time, then snaps)
- The palette is passed to the sprite editor when opening a sprite, and saved with the project file
- **Lospec import replaces** the current palette. Existing color indices remap to the same index in the new palette (index 3 stays index 3). If the new palette is shorter, elements referencing out-of-range indices fall back to index 0 (transparent)

### Eraser tool
- Click a vertex to delete it and all line segments connected to it
- Removes the vertex from the path; if the path is split into disconnected parts, they become separate elements
- **If the element has animation tracks**, show a confirmation dialog before splitting (to prevent silent data loss)
- **Track behavior on split**: Each animation track stays with whichever resulting element contains the vertex/property it references. Tracks referencing deleted vertices are dropped.

### Layer operations
- Layers are groups containing multiple elements. Elements render in creation order within a layer; layers render bottom-to-top
- **Add** new layer
- **Remove** layer
- **Combine** (merge two layers into one)
- **Move** (drag to reorder)
- Visibility toggle, lock toggle

### Fill tool
- Click a **closed** element to set its `fillColorIndex` to the active palette color
- Click **empty canvas or inside an open path** to set the sprite's `backgroundColorIndex` (like Paint's bucket fill)

### Select tool
- Click to select, shift-click for multi-select, drag for marquee selection
- Ctrl+C/V for copy/paste, Delete key to remove
- Drag to move, handles for scale/rotate (pivot = element's user-defined origin, snaps to grid)

### Palette constraints
- Max 256 colors. Index 0 = transparent/none
- When the limit is reached, show a toast notification ("Palette full — 256 color maximum")
- Sprites require project context to open (no standalone palette)

### Isometric grid
- 2:1 pixel ratio (26.57°), standard pixel-art isometric
- Snapping follows iso-grid intersections

### Undo/Redo
- Single shared stack for all mutations (drawing + animation)

### Autosave
- Debounced save 500ms after last change, plus save on tab switch and app blur
- No "unsaved changes" dialogs — undo stack handles mistakes

### Navigation
- **Project overview** is always the first tab
- **Double-click** a sprite card to open it in a new editor tab
- Multiple sprites can be open simultaneously as tabs

### New Sprite dialog
- Canvas size presets: 64x64, 128x128, 256x256, 512x512
- Freeform width/height input for custom sizes
- Name field

---

## Workspace Structure

```
messy-grapefruit/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs, lib.rs
│       ├── commands/        (file.rs, export.rs, palette.rs, watcher.rs)
│       ├── models/          (project.rs, animation.rs, palette.rs, export.rs)
│       └── export/          (svg_gen.rs, rasterize.rs, spritesheet.rs, ron_meta.rs)
├── src/
│   ├── index.html, app.tsx
│   ├── lib/                 (types.ts, tauri.ts, math.ts, history.ts, constants.ts)
│   ├── stores/              (project.ts, editor.ts, palette.ts, animation.ts, preferences.ts)
│   ├── engine/
│   │   ├── canvas.ts        (rendering loop, viewport pan/zoom)
│   │   ├── grid.ts          (standard + isometric dot grid, adaptive sizing)
│   │   ├── snap.ts          (grid snapping)
│   │   ├── hit-test.ts      (isPointInStroke/isPointInPath)
│   │   ├── merge.ts         (auto-merge coincident vertices)
│   │   └── tools/           (base.ts, line.ts, select.ts, fill.ts, eraser.ts)
│   ├── components/
│   │   ├── layout/          (AppShell, Toolbar, SidePanel, StatusBar)
│   │   ├── canvas/          (CanvasView, CanvasOverlay — curve handles, selection)
│   │   ├── sidebar/         (ToolOptionsPanel, LineToolOptions, SelectToolOptions, FillToolOptions, SettingsPanel)
│   │   ├── panels/          (LayerPanel, PalettePanel, AnimationPanel)
│   │   ├── timeline/        (Timeline, KeyframeTrack, Playhead, CurveEditor)
│   │   ├── palette/         (ColorPicker, LospecImporter, PaletteSwatch)
│   │   ├── project/         (ProjectOverview, SpriteCard, NewSpriteDialog)
│   │   └── shared/          (IconButton, NumberInput, Toggle, Dialog)
│   ├── assets/              (SVG icons — Material Design, viewBox 0 -960 960 960, fill="currentColor")
│   ├── pages/               (ProjectPage, EditorPage)
│   └── styles/              (theme.css, global.css)
├── package.json, vite.config.ts, tsconfig.json
└── .gitignore
```

---

## Tauri Command API

| Command | Purpose |
|---------|---------|
| `new_project` / `open_project` / `save_project` | Project file CRUD |
| `new_sprite` / `open_sprite` / `save_sprite` | Sprite file CRUD |
| `export_sprite(sprite, animation, output_dir, fps)` | Export one animation → PNG + RON |
| `export_all(project, sprites)` | Export all sprites' animations |
| `fetch_lospec_palette(slug)` | Fetch palette from lospec.com JSON API |
| `start_watcher(watch_dir, output_dir)` / `stop_watcher` | File watcher for auto re-export |

---

## Export Pipeline

```
Sprite + AnimationSequence + time t
  → Evaluate all PropertyTracks at t (interpolate values, apply easing)
  → Build SVG string from transformed elements + resolved palette colors
  → Rasterize SVG → PNG via resvg/usvg/tiny_skia
  → Repeat for each frame (duration / fps)
  → Fill frame background with sprite's backgroundColorIndex (if non-transparent)
  → Pack frames into spritesheet atlas (uniform NxM grid layout)
  → Write atlas.png + metadata.ron (Bevy TextureAtlasLayout-compatible)
```

RON metadata maps directly to Bevy's `TextureAtlasLayout::from_grid()` — includes sprite name, texture path, tile size, columns, rows, frame count, fps, looping flag.

Auto-export triggers on save. Watcher mode re-exports **only the changed sprite** on `.sprite` file changes. Bevy hot-reloads from the output directory.

---

## Undo/Redo

Command pattern — every mutation wraps in `{ execute(), undo() }` pushed to a single shared history stack (drawing + animation edits combined). SolidJS store mutations go through this. Ctrl+Z/Ctrl+Y navigate the stack. The redo stack clears on new actions.

---

## Theme Colors

**Dark mode (Twilight 5)**: `#292831` (bg), `#333f58` (panels), `#4a7a96` (accent), `#ee8695` (secondary), `#fbbbad` (text)

**Light mode (Golden Sunset)**: `#25213e` (bg), `#7a4a5a` (panels), `#cb765c` (accent), `#ffb873` (secondary), `#ffecd6` (text)

---

## UI Style

- **Compact, icon-driven controls** — prefer icons over text labels; text only for values and section headers
- **Sliders with numeric displays** for continuous values (stroke width, zoom, rotation, grid size)
- **Small inline color swatches** — not large color pickers; swatches show palette colors directly
- **Minimal chrome** — panels feel lightweight, not heavy dialog boxes; thin borders, subtle separators
- **Vertically stacked tool options** in sidebar — each option on its own row, not a dense property grid
- **Icons**: 30 Material Design SVGs in `src/assets/`, using `fill="currentColor"` so they inherit theme text color via CSS. Covers tools, layer controls, animation controls, actions, and theme toggle

### Hybrid Sidebar Layout

The right sidebar has two zones:
- **Top zone (context-sensitive):** Content changes based on the active tool/mode
  - *Line tool* → stroke width slider, curve/straight toggle, active color
  - *Select tool* → position, rotation, scale, origin point
  - *Fill tool* → active color selector
  - *Eraser tool* → (minimal or empty)
  - *Settings mode* → palette management, theme toggle, grid config
- **Bottom zone (fixed tabs):** Always-visible regardless of active tool
  - *Layers tab* → layer list with visibility/lock toggles, add/remove/combine/reorder
  - *Palette tab* → color swatches, add/delete, Lospec importer

---

## Implementation Phases

### Phase 1: Foundation
- Init Tauri v2 (latest stable) + SolidJS + Vite project
- Rust data models with serde (including `id` on PathVertex, `origin` on StrokeElement, `backgroundColorIndex` on Sprite)
- Basic Tauri commands: save/open/new sprite
- TypeScript types mirroring Rust
- AppShell layout with CSS Grid (canvas + top toolbar + hybrid right sidebar + status bar)
- Hybrid sidebar shell: context-sensitive top zone + fixed-tab bottom zone
- Canvas rendering loop with viewport pan/zoom
- Standard dot grid with adaptive zoom-based sizing
- Grid snapping
- **Line tool**: click to place vertices, auto-curve default (Catmull-Rom, duplicated endpoints), double-click to finish
- **Curve handles**: show/drag cp1/cp2 on selected vertices
- **Auto-merge**: detect and fuse elements when vertices coincide on same layer

### Phase 2: Drawing Completeness
- Straight/curve hotkey toggle (`C` key)
- Isometric grid mode (2:1 ratio, 26.57°)
- Select tool (click, shift-click multi-select, drag marquee, Ctrl+C/V, Delete, scale/rotate with user-defined origin)
- Fill bucket tool (closed elements → fillColorIndex, empty canvas/open paths → backgroundColorIndex)
- Eraser tool: click a vertex to delete it and all connected line segments (splits path if needed, warns if element has animation tracks)
- Palette panel (fixed tab, bottom zone): color swatches + RGB picker + add/delete colors (max 256)
- Lospec importer (`fetch_lospec_palette` command)
- Indexed color rendering
- Layer panel (fixed tab, bottom zone): **add**, **remove**, **combine**, **move** (drag reorder), visibility, lock
- Context-sensitive tool options (top zone): stroke width slider, color index, origin point — content swaps per active tool
- Undo/redo wired to all mutations (single shared stack)
- Dark/light theme toggle

### Phase 3: Animation System
- Timeline component with time axis, tracks, playhead
- Keyframe track per property (tracks reference vertex IDs, not indices)
- Animation player controls: **play/pause**, **start over** (jump to frame 0), **skip backward** (jump to previous keyframe), **skip forward** (jump to next keyframe), loop toggle
- Keyframe interpolation (linear + cubic bezier easing)
- Canvas renderer wired to animation currentTime
- Curve editor (visual bezier with draggable control points)
- Easing presets (linear, ease-in/out, bounce, elastic)
- Vertex position animation (stable vertex IDs)
- Color index step animation (hold-previous interpolation)
- Rotation/scale animation uses element's user-defined origin as pivot

### Phase 4: Export Pipeline
- `svg_gen.rs`: Sprite + time → SVG string (with backgroundColorIndex fill)
- `rasterize.rs`: SVG → PNG via resvg
- `spritesheet.rs`: frames → uniform grid atlas PNG
- `ron_meta.rs`: generate Bevy TextureAtlasLayout-compatible RON metadata
- Wire export commands
- Auto-export on save
- File watcher with `notify` crate (re-exports only the changed sprite)

### Phase 5: Project Overview & Polish
- Project overview page (freeform dashboard — 2D canvas with draggable sprite thumbnails for organization, no game-spatial meaning)
- Sprite arrangement (position, rotation, z-order for dashboard layout)
- Project file save/load (sprites require project context)
- New sprite dialog
- Keyboard shortcuts
- File dialogs (Tauri dialog plugin)
- UI polish

---

## Verification

1. **Drawing**: Open app → see dot grid → draw lines with auto-curve → verify snap to grid → drag curve handles → confirm auto-merge fuses elements when vertices coincide → verify vertex IDs are stable after merge
2. **Palette**: Import lospec palette by slug → draw with indexed colors → change a palette color → verify all art using that index updates → verify 256 color max is enforced
3. **Layers**: Add layers → draw on different layers → toggle visibility → reorder → combine → verify rendering order
4. **Selection**: Click to select → shift-click multi-select → drag marquee → Ctrl+C/V copy/paste → Delete to remove → verify origin point is draggable and grid-snapped
5. **Fill**: Fill closed path → verify fillColorIndex set → fill empty canvas → verify backgroundColorIndex set → verify background renders in export
6. **Eraser**: Delete mid-path vertex → verify path splits into two elements → try erasing vertex on animated element → verify confirmation dialog appears → confirm split → verify tracks stay with correct elements
7. **Animation**: Add keyframes on a property → set different easing presets → play animation → use skip forward/backward → verify interpolation and curve editor → verify color index uses hold-previous → verify rotation/scale pivots around origin → move playhead to non-zero time → draw new element → verify it has a visibility track (hidden before, visible after)
8. **Lospec import**: Import a palette → verify it replaces the current one → verify existing elements remap by index → import a shorter palette → verify out-of-range indices fall back to transparent
9. **Export**: Save sprite → check output directory for PNG spritesheet + RON file → verify uniform grid layout → verify RON is Bevy TextureAtlasLayout-compatible → test in a Bevy project with hot-reload
10. **Autosave**: Make changes → wait 500ms → verify file saved automatically → switch tabs → verify save triggers → verify no "unsaved changes" dialogs
11. **Navigation**: Double-click sprite card → verify editor tab opens → open multiple sprites → verify tabs work → verify project overview stays as first tab
12. **Watcher**: Start watcher → modify and save a .sprite file externally → verify only that sprite re-exports (not all sprites)
