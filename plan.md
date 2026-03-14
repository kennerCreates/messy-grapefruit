# SVG Sprite Drawing & Animation Tool

## Context

Building a Tauri desktop app (Rust backend + SolidJS frontend) for creating animated SVG sprites for a 2D isometric Bevy game. The tool draws vector art using lines/curves with an indexed color palette, animates via keyframes with editable easing curves, and auto-exports PNG spritesheets + RON metadata that Bevy hot-reloads.

The repo is currently empty (just a `.gitignore` for Rust).

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
├── sprites: ProjectSpriteRef[] (id, filePath, position, rotation, zOrder)
└── editorPreferences (theme, gridSize, gridMode, showGrid)

Sprite (.sprite file)
├── id, name, canvasWidth, canvasHeight
├── layers: Layer[]
│   └── Layer { id, name, visible, locked, elements: Element[] }
│       └── StrokeElement { id, vertices: PathVertex[], closed, strokeWidth,
│           strokeColorIndex, fillColorIndex, position, rotation, scale }
│           └── PathVertex { pos: Vec2, cp1?: Vec2, cp2?: Vec2 }
│               (cp1/cp2 = cubic bezier handles; absent = straight line)
└── animations: AnimationSequence[]
    └── AnimationSequence { id, name, duration, looping, tracks: PropertyTrack[] }
        └── PropertyTrack { property, elementId, layerId, keyframes: Keyframe[] }
            └── Keyframe { id, time, value, easing: EasingCurve }
                └── EasingCurve { preset, controlPoints: [x1,y1,x2,y2] }
```

**Animatable properties**: `position.x`, `position.y`, `rotation`, `scale.x`, `scale.y`, `strokeColorIndex`, `fillColorIndex`, `vertex.{n}.x`, `vertex.{n}.y`

---

## Key Behaviors

### Auto-merge vertices
When a vertex is placed at the same grid position as an existing vertex on the same layer, they automatically merge into a shared vertex. This enables connected paths and joined shapes. The merge is based on exact grid-snapped coordinates.

### Curve handles & straight/curve toggle
- **Auto-curve is the default**: When placing vertices, control points are auto-generated using Catmull-Rom → cubic bezier conversion
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
- Color index animation uses step interpolation (no blending between indices)
- The palette is passed to the sprite editor when opening a sprite, and saved with the project file

### Eraser tool
- Click a vertex to delete it and all line segments connected to it
- Removes the vertex from the path; if the path is split into disconnected parts, they become separate elements

### Layer operations
- **Add** new layer
- **Remove** layer
- **Combine** (merge two layers into one)
- **Move** (drag to reorder)
- Visibility toggle, lock toggle

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
│   │   ├── panels/          (LayerPanel, PalettePanel, PropertiesPanel, AnimationPanel)
│   │   ├── timeline/        (Timeline, KeyframeTrack, Playhead, CurveEditor)
│   │   ├── palette/         (ColorPicker, LospecImporter, PaletteSwatch)
│   │   ├── project/         (ProjectOverview, SpriteCard, NewSpriteDialog)
│   │   └── shared/          (IconButton, NumberInput, Toggle, Dialog)
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
  → Pack frames into spritesheet atlas (grid layout)
  → Write atlas.png + metadata.ron
```

Auto-export triggers on save. Watcher mode re-exports on `.sprite` file changes. Bevy hot-reloads from the output directory.

---

## Undo/Redo

Command pattern — every mutation wraps in `{ execute(), undo() }` pushed to a history stack. SolidJS store mutations go through this. Ctrl+Z/Ctrl+Y navigate the stack. The redo stack clears on new actions.

---

## Theme Colors

**Dark mode (Twilight 5)**: `#292831` (bg), `#333f58` (panels), `#4a7a96` (accent), `#ee8695` (secondary), `#fbbbad` (text)

**Light mode (Golden Sunset)**: `#25213e` (bg), `#7a4a5a` (panels), `#cb765c` (accent), `#ffb873` (secondary), `#ffecd6` (text)

---

## Implementation Phases

### Phase 1: Foundation
- Init Tauri v2 + SolidJS + Vite project
- Rust data models with serde
- Basic Tauri commands: save/open/new sprite
- TypeScript types mirroring Rust
- AppShell layout with CSS Grid
- Canvas rendering loop with viewport pan/zoom
- Standard dot grid with adaptive zoom-based sizing
- Grid snapping
- **Line tool**: click to place vertices, auto-curve default, double-click to finish
- **Curve handles**: show/drag cp1/cp2 on selected vertices
- **Auto-merge**: detect and merge coincident vertices on same layer

### Phase 2: Drawing Completeness
- Straight/curve hotkey toggle (`C` key)
- Isometric grid mode
- Select tool (click, drag move, scale/rotate handles)
- Fill bucket tool
- Eraser tool: click a vertex to delete it and all connected line segments (splits path if needed)
- Palette panel + RGB color picker + add/delete colors
- Lospec importer (`fetch_lospec_palette` command)
- Indexed color rendering
- Layer panel: **add**, **remove**, **combine**, **move** (drag reorder), visibility, lock
- Properties panel (stroke width, color index)
- Undo/redo wired to all mutations
- Dark/light theme toggle

### Phase 3: Animation System
- Timeline component with time axis, tracks, playhead
- Keyframe track per property
- Animation player controls: **play/pause**, **start over** (jump to frame 0), **skip backward** (jump to previous keyframe), **skip forward** (jump to next keyframe), loop toggle
- Keyframe interpolation (linear + cubic bezier easing)
- Canvas renderer wired to animation currentTime
- Curve editor (visual bezier with draggable control points)
- Easing presets (linear, ease-in/out, bounce, elastic)
- Vertex position animation
- Color index step animation

### Phase 4: Export Pipeline
- `svg_gen.rs`: Sprite + time → SVG string
- `rasterize.rs`: SVG → PNG via resvg
- `spritesheet.rs`: frames → atlas PNG
- `ron_meta.rs`: generate RON metadata
- Wire export commands
- Auto-export on save
- File watcher with `notify` crate

### Phase 5: Project Overview & Polish
- Project overview page (2D canvas with sprite thumbnails)
- Sprite arrangement (position, rotation, z-order layers)
- Project file save/load
- New sprite dialog
- Keyboard shortcuts
- File dialogs (Tauri dialog plugin)
- UI polish

---

## Verification

1. **Drawing**: Open app → see dot grid → draw lines with auto-curve → verify snap to grid → drag curve handles → confirm auto-merge when vertices coincide
2. **Palette**: Import lospec palette by slug → draw with indexed colors → change a palette color → verify all art using that index updates
3. **Layers**: Add layers → draw on different layers → toggle visibility → reorder → combine → verify rendering order
4. **Animation**: Add keyframes on a property → set different easing presets → play animation → use skip forward/backward → verify interpolation and curve editor
5. **Export**: Save sprite → check output directory for PNG spritesheet + RON file → verify frames are correct → test in a Bevy project with hot-reload
6. **Watcher**: Start watcher → modify and save a .sprite file externally → verify re-export triggers automatically
