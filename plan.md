# SVG Sprite Drawing & Animation Tool

## Context

Building a native Rust desktop app (eframe/egui) for creating animated SVG sprites for a 2D isometric Bevy game. The art style targets **high-resolution isometric line art** (similar to *They Are Billions*), not pixel art. The tool draws vector art using lines/curves with an indexed color palette, animates via keyframes with editable easing curves, and exports runtime bone animation data (RON) + texture atlases that Bevy hot-reloads.

---

## Architecture

- **eframe + egui** — Pure Rust desktop app. egui immediate-mode UI for panels, toolbars, and controls. egui's `Painter` API for the canvas drawing surface (bezier curves, strokes, shapes, hit-testing)
- **Rust** — All logic in Rust: file I/O, SVG generation, PNG rasterization (`resvg`), texture atlas packing, file watching (`notify`), RON export
- **JSON file format** — `.sprite` (per sprite) and `.spriteproj` (project overview). Human-readable, diffable, trivial serde

### Why these choices
- **eframe/egui**: Single-language (Rust), no web stack complexity, immediate-mode UI is natural for a drawing/animation tool with frequent repaints. egui's `Painter` provides direct drawing primitives (bezier curves, strokes, shapes).
- **JSON over binary**: Sprite files are small (vector data). Human-readable, version-controllable, diffable.

---

## Data Model

### Core Types (Rust with serde)

```
Project (.spriteproj file)
├── name, formatVersion, exportDir
├── palette: Palette { name, colors: PaletteColor[] }  // index 0 = transparent, shared by ALL sprites
├── sprites: ProjectSpriteRef[] (id, filePath, position, rotation, zOrder, selectedAnimationId?, selectedSkinId?)  // layout + preview state for dashboard, not game-spatial
├── exportSettings: ExportSettings { mode: "bone"|"spritesheet", fps, layout: "row"|"column"|"grid", trim, padding }
└── editorPreferences (theme, gridSize, gridMode, showGrid)

Sprite (.sprite file)  // canvasWidth/canvasHeight = export pixel dimensions (1:1, no scale factor)
├── id, name, formatVersion, canvasWidth, canvasHeight, backgroundColorIndex (default 0/transparent)
├── layers: Layer[]
│   └── Layer { id, name, visible, locked, elements: Element[],
│         socket?: { parentElementId, parentVertexId } }  // if set, layer follows this vertex
│       └── constraints?: LayerConstraints
│       └── Element = StrokeElement | IKTargetElement
│           StrokeElement { id, name?, type: "stroke", vertices: PathVertex[], closed, strokeWidth,
│               strokeColorIndex, fillColorIndex, position, rotation, scale, origin: Vec2 }
│           IKTargetElement { id, name?, type: "ik-target", position: Vec2, ikChainId: string }
│               (lightweight — no vertices/strokes, renders as crosshair icon on canvas)
│               (position is world-space — not relative to the layer. Lives on tip layer for organization only)
│           └── PathVertex { id, pos: Vec2, cp1?: Vec2, cp2?: Vec2 }
│               (cp1/cp2 = cubic bezier handles; absent = straight line)
│               (origin = user-defined pivot point for rotation/scale, snaps to grid)
│
│   LayerConstraints {
│     volumePreserve?: boolean,                          // scale_x = 1/scale_y
│     lookAt?: { targetElementId, targetVertexId?,       // aim at element origin or specific vertex
│       restAngle, minAngle, maxAngle, mix, smooth?: { frequency, damping } }
│     physics?: { frequency, damping, mix,               // spring follow
│       gravity?: { angle, strength },                   // constant force (angle in degrees, 270 = down)
│       wind?: { strength, frequency } }                 // sinusoidal force
│     procedural?: ProceduralModifier[]                  // additive oscillation
│   }
│   ProceduralModifier { property, waveform: "sine"|"noise", amplitude, frequency, phase, blend: "additive"|"multiplicative" }
├── skins: Skin[]
│   └── Skin { id, name, overrides: SkinOverride[] }
│       └── SkinOverride { elementId, strokeColorIndex?, fillColorIndex?, strokeWidth? }
│           // Each override replaces visual properties on a specific element
│           // Omitted fields inherit from the base element
│           // The base sprite (no skin applied) is the implicit "default" skin
└── animations: AnimationSequence[]
    └── AnimationSequence { id, name, duration, looping, tracks: PropertyTrack[], ikChains: IKChain[] }
        // duration auto-extends when a keyframe is placed past the current end; also manually editable (e.g., to add trailing hold time)
        └── PropertyTrack { property, elementId, layerId, keyframes: Keyframe[] }
            └── Keyframe { id, time, value, easing: EasingCurve }
                └── EasingCurve { preset, controlPoints: [x1,y1,x2,y2] }
        └── IKChain { id, name, layerIds: string[],       // ordered root→tip socket chain
              targetElementId: string,                       // references an IKTargetElement (keyframe its position via PropertyTrack)
              mix: number,                                   // 0=FK, 1=IK, keyframeable via PropertyTrack on the IKTargetElement
              bendDirection: 1|-1,                           // sign flip for 2-bone
              solver: "two-bone"|"fabrik",                   // analytical or iterative
              angleConstraints?: { layerId, min, max }[] }   // per-joint angle limits (2-bone only initially)
```

**Animatable properties**: `position.x`, `position.y`, `rotation`, `scale.x`, `scale.y`, `strokeColorIndex`, `fillColorIndex`, `vertex.{id}.x`, `vertex.{id}.y`, `visible`

*Vertex animation uses stable vertex IDs (not positional indices) so tracks survive vertex insertion/deletion.*

**Visibility & drawing on non-zero frames**: Elements have a `visible` property (animatable, hold-previous interpolation). Drawing a new element while the playhead is at a non-zero time auto-creates a visibility track: hidden before the current time, visible from the current time onward. Hidden elements are excluded from the evaluation pipeline entirely — no IK, physics, or constraints until visible.

**Rest pose**: Frame 0 with no animation playing is the canonical rest/bind pose. All element positions, rotations, scales, and vertex positions at frame 0 define the default state. IK bone lengths are computed from the rest pose (distance from socket vertex to child layer's origin). The export pipeline uses the rest pose as the reference for default transforms and bone setup. Editing the sprite with the playhead at frame 0 and no sequence selected modifies the rest pose directly.

---

## Key Behaviors

### Viewport controls
- **Zoom**: Scroll wheel (centered on cursor)
- **Pan**: Middle-click drag

### Auto-merge vertices
When a vertex is placed at the same grid position as an existing vertex on the same layer, the elements **fuse into a single StrokeElement** with a combined vertex list. This enables connected paths and joined shapes. The merge is based on exact grid-snapped coordinates. Cross-element merges fuse the two elements; same-element merges close the path. **Property resolution**: the target (existing) element's properties win — `strokeWidth`, `strokeColorIndex`, `fillColorIndex`, `position`, `rotation`, `scale`, and `origin` are kept from the element being merged into. **Animation tracks**: if the absorbed element has animation tracks, a confirmation dialog warns before merging — absorbed element's tracks are dropped (target element's tracks are kept). Undo captures the pre-merge state of both elements so the merge can be fully reversed.

**Visual merge preview**: When placing a vertex near an existing vertex that would trigger a merge, the target vertex/element highlights and a snap indicator appears. This makes the merge behavior predictable and avoids surprise fusions.

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
- Removes the vertex from the path; if the path is split into disconnected parts, they become separate elements. Both resulting elements inherit `position`, `rotation`, `scale`, `origin`, and color indices from the original
- **If the element has animation tracks**, show a confirmation dialog before splitting (to prevent silent data loss)
- **Track behavior on split**: Each animation track stays with whichever resulting element contains the vertex/property it references. Tracks referencing deleted vertices are dropped.

### Layer operations
- Layers are groups containing multiple elements. Elements render in creation order within a layer; layers render bottom-to-top
- **Add** new layer
- **Remove** layer
- **Duplicate** layer (deep-copies all elements with new IDs. Animation tracks, socket references, and constraints are not copied)
- **Mirror** layer (flip all elements horizontally or vertically around the bounding box center of the layer's elements. Flips vertex positions and control points. Useful for creating symmetrical body parts — e.g., duplicate left arm, mirror to make right arm)
- **Combine** (merge two layers into one). If either layer is socketed, the combined layer keeps the socket of the *top* layer. If only the bottom layer was socketed, a warning dialog is shown before proceeding (socket will be dropped). If either layer is a socket parent for other layers, those child references update to point to the combined layer
- **Move** (drag to reorder)
- Visibility toggle, lock toggle

### Fill tool
- Click a **closed** element to set its `fillColorIndex` to the active palette color
- Click **empty canvas or inside an open path** to set the sprite's `backgroundColorIndex` (like Paint's bucket fill)

### Select tool
- Click to select, shift-click for multi-select, drag for marquee selection, Ctrl+A to select all elements on unlocked layers
- Ctrl+C/V for copy/paste, Delete key to remove
- Drag to move, handles for scale/rotate (pivot = element's user-defined origin, snaps to grid)
- **Copy/paste**: Paste creates a new layer containing copies of the selected elements. All pasted elements get new IDs. Animation tracks, socket references, and layer constraints are not copied (constraints reference other elements/layers by ID and would break). Pasted layer is positioned with a small offset (+10, +10) from the original. **Cross-sprite paste**: Elements are serialized to the system clipboard as JSON, so copy/paste works across sprite tabs. Color indices reference the shared project palette, so colors stay consistent

### Palette constraints
- Max 256 colors. Index 0 = transparent/none
- When the limit is reached, show a toast notification ("Palette full — 256 color maximum")
- Sprites require project context to open (no standalone palette)

### Isometric grid
- 2:1 pixel ratio (26.57°), standard isometric
- Snapping follows iso-grid intersections

### Layer sockets (transform parenting)
- A layer can be **socketed** to a vertex on any element in another layer. The socketed layer inherits the **position and rotation** of the parent vertex (scale stays independent)
- Any existing vertex can serve as a socket point — no special vertex type needed. Attach via the layer panel or a context menu
- Socket chains can be unlimited depth (arm → hand → weapon → gem). The renderer walks the chain root-to-leaf, accumulating position + rotation at each level
- Circular socket references are rejected at assignment time
- When the parent vertex is animated, the socketed layer follows automatically — no need to duplicate keyframes
- Socketed layers still have their own local position/rotation/scale (applied as offset relative to the parent vertex)
- Deleting a socket parent vertex shows a warning and detaches any socketed child layers (they snap to their current world-space position)

### Procedural Animation

**Evaluation order** (per frame, must be stepped sequentially from frame 0 due to stateful physics):
1. Evaluate FK from keyframes (interpolate all property tracks)
2. Initial socket chain walk: compute world-space positions for all joints (needed by IK solver)
3. Solve IK chains (blended with FK via per-chain `mix`). IK bone length = distance from socket vertex to child layer's origin
4. Apply constraints: look-at (atan2 + angle limits + optional spring smoothing), volume preservation (scale_x = 1/scale_y)
5. Apply procedural modifiers: additive/multiplicative sine/noise
6. Apply physics simulation: spring dynamics chase the post-modifier values as targets (semi-implicit Euler, world space). Convert result back to local space. Gravity/wind operate in world space
7. Final socket chain walk: root-to-leaf, accumulating position + rotation with all modifications applied

**Inverse Kinematics (IK)**
- **2-bone analytical solver**: Law of cosines. Covers arms/legs. Exact, no iteration. Bend direction is a +1/−1 sign flip on the offset angle
- **FABRIK solver**: For chains longer than 2 (tails, tentacles, spines). Forward-backward reaching, 3–10 iterations. Add tiny perturbation to avoid collinear deadlock
- **IK target**: A lightweight canvas element (position + crosshair icon, no vertices/strokes). Draggable on canvas, keyframeable via normal PropertyTrack. One target element per IK chain, lives on the chain's tip layer
- **FK/IK mix**: A keyframeable 0–1 parameter per chain. At 0 = pure FK keyframes, at 1 = pure IK. Animate the mix to smoothly transition mid-timeline (Spine-style)
- **Angle constraints**: Per-joint min/max angle relative to parent bone. Start with 2-bone only; skip for FABRIK initially
- **Bone length**: Distance from the socket vertex (on parent element) to the child layer's origin point. Computed from the rest pose
- IK chains are defined over sequences of socketed layers — the socket chain is the bone hierarchy

**Spring / Jiggle Physics**
- Per-layer opt-in constraint. The spring chases the layer's keyframed+IK+constraint-solved position as its target
- Parameters: **frequency** (Hz, 0.1–10, default 2), **damping** ratio (0–2, default 0.5), **mix** (0–1)
- **Gravity**: constant force with configurable angle (270° = down) and strength. Default 0 (opt-in)
- **Wind**: sinusoidal force with configurable strength and frequency. Default 0 (opt-in)
- Integration: semi-implicit Euler (`velocity += force * dt; position += velocity * dt`). Simulates in **world space** (so gravity/wind directions are absolute), result converted back to local space for the socket chain
- Spring state resets when animation restarts (snap to FK pose)

**Squash & Stretch**
- Per-layer `volumePreserve` toggle. When enabled, `scale_x = 1 / scale_y` is enforced automatically
- Works with keyframed scale, IK, and physics — applied as a post-constraint fixup
- Pivot is the element's `origin` point

**Procedural Modifiers**
- Per-layer list of additive oscillations on any animatable property
- Parameters: **waveform** (sine or Perlin noise), **amplitude**, **frequency** (Hz), **phase** (degrees), **blend** mode (additive or multiplicative)
- Good for: idle breathing (sine on scale.y ~0.25Hz), floating (sine on position.y ~0.5Hz), flickering flames (noise on rotation)
- Applied **before** physics, so spring dynamics can smooth procedural oscillation into organic secondary motion

**Look-At Constraint**
- Per-layer constraint. Layer rotates to face a target element (or a specific vertex on a target element)
- Parameters: **restAngle** (default facing direction), **minAngle/maxAngle** (rotation limits relative to rest), **mix** (0–1, keyframeable)
- Optional **spring smoothing**: instead of snapping to the target angle, smooth via damped spring (reuses frequency + damping params). Prevents mechanical snapping
- Handles angle wrapping at ±π (shortest angular difference)
- Good for: eyes tracking a point, turrets, head turns

### Skins (visual variants)
- A sprite can have multiple **skins** — named sets of visual overrides applied on top of the base element properties
- Each skin contains overrides per element: replacement `strokeColorIndex`, `fillColorIndex`, and/or `strokeWidth`. Omitted fields inherit from the base element
- The base sprite with no skin applied is the implicit "default" skin — it doesn't appear in the skins list
- **Bone structure, vertices, animations, sockets, IK chains, and constraints are shared across all skins** — only visual properties differ. Animate once, swap appearance
- Skin selector dropdown in the editor toolbar lets you preview any skin while editing
- When a skin is active in the editor, drawing/editing changes modify the **base** element (shared geometry), but the canvas renders with the skin's visual overrides so you can see the result
- **Export**: each skin produces its own texture atlas (different part PNGs), but all skins share the same animation RON data. The exported RON includes a skin manifest listing available skins and their atlas references
- Use case: same walk/attack/idle animations reused across soldier variants, enemy tiers, or equipment loadouts with different color schemes

### Undo/Redo
- Single shared stack for all mutations (drawing + animation)

### Autosave
- Debounced save 3 seconds after last change, plus save on tab switch and app blur
- No "unsaved changes" dialogs — undo stack handles mistakes
- **First save**: Creating a new project prompts for a save directory (`rfd` file dialog). New sprites are saved as `.sprite` files relative to the project directory. Autosave only activates once a file path is established

### Navigation
- **Project overview** is always the first tab
- **Double-click** a sprite card to open it in a new editor tab
- Multiple sprites can be open simultaneously as tabs

### New Sprite dialog
- Canvas size presets: 64x64, 128x128, 256x256, 512x512
- Freeform width/height input for custom sizes
- Name field

### Canvas boundary
- A dashed rectangle on the canvas shows the export area (`canvasWidth` x `canvasHeight`). Art outside this boundary is preserved but excluded from spritesheet export. Bone export rasterizes each element at its own bounding box (canvas boundary is visual reference only)
- Boundary always visible regardless of zoom, rendered in a subtle contrasting color (theme-aware)

### Canvas resize
- Change `canvasWidth`/`canvasHeight` after creation via sprite settings
- Existing art stays at its current position (no scaling or repositioning)

---

## Workspace Structure

```
messy-grapefruit/
├── Cargo.toml
├── src/
│   ├── main.rs              (eframe app entry point)
│   ├── model/
│   │   ├── mod.rs
│   │   ├── vec2.rs          (Vec2 math type with ops)
│   │   ├── project.rs       (Project, Palette, EditorPreferences)
│   │   └── sprite.rs        (Sprite, Layer, StrokeElement, PathVertex, Skin)
│   ├── state/
│   │   ├── mod.rs
│   │   ├── editor.rs        (EditorState, ViewportState, SelectionState, tools)
│   │   ├── project.rs       (ProjectState, OpenSprite, tab management)
│   │   └── history.rs       (snapshot-based undo/redo)
│   ├── io.rs                (save/load sprite/project JSON, Lospec fetch)
│   ├── math.rs              (Catmull-Rom, bezier eval/split/flatten, auto-curves)
│   ├── theme.rs             (dark/light theme colors for egui)
│   ├── ui/                  (egui UI modules — panels, toolbar, sidebar, canvas)
│   │   ├── canvas.rs        (canvas rendering, viewport pan/zoom, drawing)
│   │   ├── grid.rs          (standard + isometric dot grid, adaptive sizing)
│   │   ├── toolbar.rs       (top toolbar with tool buttons)
│   │   ├── sidebar.rs       (hybrid right sidebar — tool options + layers/palette/skins tabs)
│   │   ├── timeline.rs      (animation timeline, keyframe tracks, playhead)
│   │   └── status_bar.rs    (bottom status bar)
│   ├── engine/
│   │   ├── snap.rs          (grid snapping)
│   │   ├── hit_test.rs      (point-in-stroke/path)
│   │   ├── merge.rs         (auto-merge coincident vertices)
│   │   ├── ik.rs            (2-bone analytical + FABRIK solvers)
│   │   ├── physics.rs       (spring simulation, gravity, wind)
│   │   └── constraints.rs   (look-at, volume preserve, procedural modifiers)
│   └── export/
│       ├── svg_gen.rs        (Sprite + time → SVG string)
│       ├── rasterize.rs      (SVG → PNG via resvg)
│       ├── bone_export.rs    (element → part PNGs + animation RON)
│       ├── ron_meta.rs       (Bevy-compatible RON metadata)
│       └── spritesheet.rs    (frame atlas packing)
└── .gitignore
```

---

## Export Pipeline

### Primary: Runtime bone animation (skeletal animations)

```
Sprite
  → Rasterize each element as a separate PNG (body parts)
  → Pack part PNGs into a single texture atlas
  → Export animation data as RON:
    → Per-element: texture region, origin point, socket parent reference
    → Per-animation: keyframes with interpolation info, IK chain definitions,
      physics/constraint parameters, procedural modifier params
  → Bevy runtime component reads RON, assembles parts, evaluates animation at 60 FPS
```

Runtime bone export produces smaller textures and smooth full-framerate animation. Requires a Bevy-side runtime component that evaluates the animation pipeline (FK → IK → constraints → physics → procedural → socket transforms) — this is a separate project with its own documentation. This is the primary export path — high-res line art sprites would produce prohibitively large spritesheets at decent frame rates.

### Secondary: Spritesheet (simple assets, lower priority)

```
Sprite + AnimationSequence
  → Step sequentially from frame 0 at configurable FPS:
    → Full evaluation pipeline (FK → IK → constraints → procedural → physics → socket walk)
    → Build SVG string from transformed elements + resolved palette colors
    → Rasterize SVG → PNG via resvg/usvg/tiny_skia
    → Fill frame background with sprite's backgroundColorIndex (if non-transparent)
  → Trim transparent borders uniformly (find smallest bounding box that fits the largest frame, apply to all)
  → Pack all frames into spritesheet atlas with configurable layout and padding
  → Write atlas.png + metadata.ron (Bevy TextureAtlasLayout-compatible)
```

Useful for VFX, particles, UI elements, and simple environmental props that don't warrant a bone rig.

**Spritesheet options:**
- **FPS**: Configurable (default 12). Physics simulation always runs at 60 FPS internally — export samples frames at the configured FPS rate, so baked physics matches the preview
- **Layout**: Row (single row), column (single column), or grid (NxM, default). Determines `columns` and `rows` in the atlas
- **Trim**: Toggle (default on). Removes transparent borders uniformly — computes the smallest bounding box across all frames and crops all frames to that size. Keeps frames uniform for `from_grid()`
- **Padding**: Pixels between frames (default 1). Prevents texture bleed at edges during rendering

RON metadata includes `tile_size`, `columns`, `rows`, `padding`, and `offset` — maps directly to Bevy's `TextureAtlasLayout::from_grid()`.

### Shared behavior

- **Preview** runs at 60 FPS in the editor (matches game target)
- **Manual export**: Opens a preview dialog displaying the generated atlas image and RON metadata summary (tile size, frame count, atlas dimensions). Allows adjusting settings (FPS, layout, trim, padding) and re-previewing before confirming. Settings are saved and reused by auto-export
- **Auto-export on save**: Uses the last-used export settings, no dialog, writes directly to disk. Watcher mode re-exports **only the changed sprite** on `.sprite` file changes. Bevy hot-reloads from the output directory

---

## Undo/Redo

Snapshot-based undo — every mutation captures the full sprite state before and after. Pushed to a single shared history stack (drawing + animation edits combined). Ctrl+Z/Ctrl+Y navigate the stack. The redo stack clears on new actions.

**Physics & undo**: Undoing a physics/constraint parameter change does not rewind the playhead. Since physics only runs during playback (scrubbing shows FK-only), there is no stale simulation state — physics will re-simulate correctly from frame 0 the next time playback starts.

---

## Theme Colors

**Dark mode (Twilight 5)**: `#292831` (bg), `#333f58` (panels), `#4a7a96` (accent), `#ee8695` (secondary), `#fbbbad` (text)

**Light mode (Golden Sunset)**: `#ffecd6` (bg), `#ffb873` (panels), `#cb765c` (accent), `#7a4a5a` (secondary), `#25213e` (text)

---

## UI Style

- **Compact, icon-driven controls** — prefer icons over text labels; text only for values and section headers
- **Sliders with numeric displays** for continuous values (stroke width, zoom, rotation, grid size)
- **Small inline color swatches** — not large color pickers; swatches show palette colors directly
- **Minimal chrome** — panels feel lightweight, not heavy dialog boxes; thin borders, subtle separators
- **Vertically stacked tool options** in sidebar — each option on its own row, not a dense property grid
- **Icons**: egui built-in icons and Unicode symbols for tool/action buttons

### Hybrid Sidebar Layout

The right sidebar has two zones:
- **Top zone (context-sensitive):** Content changes based on the active tool/mode
  - *Line tool* → stroke width slider, curve/straight toggle, active color
  - *Select tool* → position, rotation, scale, origin point, constraints (IK, physics, look-at, volume preserve, procedural modifiers — shown when element has them)
  - *Fill tool* → active color selector
  - *Eraser tool* → (minimal or empty)
  - *Settings mode* → palette management, theme toggle, grid config
- **Bottom zone (fixed tabs):** Always-visible regardless of active tool
  - *Layers tab* → layer list with visibility/lock toggles, add/remove/duplicate/mirror/combine/reorder
  - *Palette tab* → color swatches, add/delete, Lospec importer
  - *Skins tab* → skin list with create/rename/duplicate/delete, per-element override editor

---

## Implementation Phases

### Phase 1: Foundation
- Init eframe/egui project with Cargo dependencies
- Rust data models with serde (including `id` on PathVertex, `origin` on StrokeElement, `backgroundColorIndex` on Sprite)
- Save/open/new sprite via `rfd` file dialogs and `serde_json`
- AppShell layout with egui panels (canvas + top toolbar + hybrid right sidebar + status bar)
- Hybrid sidebar shell: context-sensitive top zone + fixed-tab bottom zone
- Canvas boundary rendering (dashed rectangle at canvasWidth x canvasHeight)
- Canvas rendering with egui `Painter` + viewport pan/zoom
- Standard dot grid with adaptive zoom-based sizing
- Grid snapping
- **Line tool**: click to place vertices, auto-curve default (Catmull-Rom, duplicated endpoints), double-click to finish
- **Curve handles**: show/drag cp1/cp2 on selected vertices
- **Auto-merge**: detect and fuse elements when vertices coincide on same layer, with visual preview indicator (highlight target vertex/element before merge)

### Phase 2: Drawing Completeness
- Straight/curve hotkey toggle (`C` key)
- Isometric grid mode (2:1 ratio, 26.57°)
- Select tool (click, shift-click multi-select, drag marquee, Ctrl+A select all, Ctrl+C/V copy/paste including cross-sprite, Delete, scale/rotate with user-defined origin)
- Fill bucket tool (closed elements → fillColorIndex, empty canvas/open paths → backgroundColorIndex)
- Eraser tool: click a vertex to delete it and all connected line segments (splits path if needed, warns if element has animation tracks)
- Palette panel (fixed tab, bottom zone): color swatches + RGB picker + add/delete colors (max 256)
- Lospec importer (blocking HTTP fetch via `reqwest`)
- Indexed color rendering
- Layer panel (fixed tab, bottom zone): **add**, **remove**, **duplicate**, **mirror**, **combine**, **move** (drag reorder), visibility, lock
- Context-sensitive tool options (top zone): stroke width slider, color index, origin point — content swaps per active tool
- Undo/redo wired to all mutations (single shared stack)
- Dark/light theme toggle

### Phase 3: Animation System
- Timeline component with time axis, tracks, playhead
- **Animation sequence tabs** at top of timeline panel: click to switch, right-click to rename/delete, + button to create new sequence
- Keyframe track per property (tracks reference vertex IDs, not indices)
- Animation player controls: **play/pause**, **start over** (jump to frame 0), **skip backward** (jump to previous keyframe), **skip forward** (jump to next keyframe), loop toggle
- Preview playback at 60 FPS (physics/spring simulation only runs during playback; scrubbing the timeline shows FK-only pose)
- Keyframe interpolation (linear + cubic bezier easing)
- Canvas renderer wired to animation currentTime
- Curve editor (visual bezier with draggable control points)
- Easing presets (linear, ease-in/out, bounce, elastic)
- Vertex position animation (stable vertex IDs)
- Color index step animation (hold-previous interpolation)
- Rotation/scale animation uses element's user-defined origin as pivot
- **Onion skinning**: toggle to show ghost frames before/after the current frame. Configurable number of frames (default 2 before, 2 after). Ghost frames rendered with reduced opacity. Useful for timing and spacing

### Phase 4: Layer Sockets
- **Layer sockets**: attach a layer to a parent vertex, inherit position + rotation, unlimited chain depth. Socket UI in layer panel. Circular reference detection. Warning on parent vertex deletion

### Phase 5: Skins
- Skin data model: `Skin { id, name, overrides: SkinOverride[] }` per sprite
- `SkinOverride { elementId, strokeColorIndex?, fillColorIndex?, strokeWidth? }` — omitted fields inherit from base element
- Skin management panel in sidebar: create, rename, duplicate, delete skins
- Per-skin override editor: select an element, toggle which visual properties this skin overrides, set override values
- Skin selector dropdown in editor toolbar for previewing skins while editing
- Canvas renders with active skin's overrides applied; drawing/editing always modifies the shared base geometry
- Undo/redo support for all skin mutations (create, delete, modify overrides)
- Export integration: each skin produces a separate texture atlas, all skins share the same animation RON. RON includes a skin manifest mapping skin names to atlas references

### Phase 6: Inverse Kinematics
- **2-bone analytical IK solver**: law of cosines, bend direction sign flip, keyframeable target position + mix
- **FABRIK solver**: forward-backward reaching for chains > 2, perturbation for collinear cases
- IK chain definition UI: select socketed layers to form a chain, set solver type
- IK target as draggable canvas point, keyframeable on the timeline
- Per-joint angle constraints (min/max) for 2-bone chains
- FK/IK mix wired to evaluation pipeline (FK → socket walk → IK → final socket walk)
- Unit tests for IK solver math (law of cosines, FABRIK convergence, angle constraints, bend direction)

### Phase 7: Constraints & Dynamics
- **Spring/jiggle physics**: per-layer constraint with frequency, damping, mix sliders. Semi-implicit Euler integration
- **Gravity + wind**: per-physics-constraint forces, gravity angle/strength, wind strength/frequency. Default 0
- Spring state reset on animation restart
- **Squash & stretch**: per-layer volume-preserve toggle, scale_x = 1/scale_y
- **Procedural modifiers**: per-layer sine/noise oscillation on any property. Amplitude, frequency, phase, blend mode
- **Look-at constraint**: per-layer aim at target element/vertex, rest angle, angle limits, mix, optional spring smoothing
- Full evaluation pipeline wired in correct order: FK → IK → constraints → procedural → physics → socket transforms
- Constraint parameters exposed in the layer panel and select tool's context-sensitive sidebar panel
- **Visual debug overlays**: render bone chains, IK targets, constraint gizmos, spring targets as toggleable canvas overlays (for authoring and debugging)
- Unit tests for spring integrator, angle wrapping, Catmull-Rom conversion, procedural waveforms

### Phase 8: Export Pipeline
- `svg_gen.rs`: Sprite + time → SVG string (with backgroundColorIndex fill)
- `rasterize.rs`: SVG → PNG via resvg
- `bone_export.rs`: element → individual part PNGs + animation data RON for runtime bone mode (primary export path). Skin-aware: export one atlas per skin, shared animation RON with skin manifest
- `ron_meta.rs`: generate Bevy-compatible RON metadata
- Export preview dialog: show atlas preview + RON metadata summary, adjust settings before confirming
- Wire export commands
- Auto-export on save: exports all animations for the changed sprite using last-used settings, no dialog
- File watcher with `notify` crate (re-exports only the changed sprite)

### Phase 9: Project Overview & Polish
- Project overview page (live compose preview — 2D canvas with draggable sprites for previewing how they look together)
- Each sprite on the dashboard renders its animation live (not static thumbnails), with per-sprite dropdowns to select which animation sequence and skin to preview
- Sprite arrangement (position, rotation, z-order for dashboard layout)
- Project file save/load (sprites require project context)
- New sprite dialog
- Keyboard shortcuts
- File dialogs (`rfd` crate)
- **Spritesheet export** (secondary, lower priority): `spritesheet.rs` for simple assets (VFX, particles, props). Configurable FPS, layout (row/column/grid), uniform trim toggle, padding. Exports atlas PNG + TextureAtlasLayout RON via `from_grid()`
- UI polish

---

## Testing Strategy

- **Unit tests on engine math**: IK solvers (law of cosines, FABRIK convergence, angle constraints, bend direction), spring integrator (convergence, energy conservation), angle wrapping (±π), Catmull-Rom → cubic bezier conversion, procedural waveform generators. These are pure functions — easy to test, high regression value.
- **Visual debug overlays**: Toggleable canvas overlays that render bone chains, IK targets, constraint gizmos, and spring targets. Not automated, but essential for authoring and debugging procedural animation. Built during Phase 7.
- **Round-trip save/load tests**: If serialization bugs appear, add targeted tests for `.sprite` / `.spriteproj` round-trips via serde.

---

## Verification

1. **Drawing**: Open app → see dot grid → draw lines with auto-curve → verify snap to grid → drag curve handles → approach an existing vertex → verify merge preview highlights target → confirm auto-merge fuses elements when vertices coincide → verify vertex IDs are stable after merge
2. **Palette**: Import lospec palette by slug → draw with indexed colors → change a palette color → verify all art using that index updates → verify 256 color max is enforced
3. **Layers**: Add layers → draw on different layers → toggle visibility → reorder → combine → duplicate → mirror horizontally → verify rendering order
4. **Selection**: Click to select → shift-click multi-select → Ctrl+A select all → drag marquee → Ctrl+C/V copy/paste (including cross-sprite paste) → Delete to remove → verify origin point is draggable and grid-snapped
5. **Fill**: Fill closed path → verify fillColorIndex set → fill empty canvas → verify backgroundColorIndex set → verify background renders in export
6. **Eraser**: Delete mid-path vertex → verify path splits into two elements → try erasing vertex on animated element → verify confirmation dialog appears → confirm split → verify tracks stay with correct elements
7. **Animation**: Add keyframes on a property → set different easing presets → play animation → use skip forward/backward → verify interpolation and curve editor → verify color index uses hold-previous → verify rotation/scale pivots around origin → place keyframe past duration → verify duration auto-extends → move playhead to non-zero time → draw new element → verify it has a visibility track (hidden before, visible after)
8. **Layer sockets**: Draw an arm element → draw a weapon on a separate layer → socket the weapon layer to a vertex on the arm → animate the arm → verify weapon follows → chain a third layer to the weapon → verify full chain works → try creating a circular reference → verify it's rejected → delete the socket vertex → verify warning and child detaches to world-space position
9. **IK**: Create a 2-bone socket chain (upper arm → forearm → hand) → set up IK constraint → drag IK target → verify joints solve correctly → flip bend direction → verify elbow flips → animate IK target position → play → verify smooth tracking → animate mix from 0→1 → verify FK-to-IK transition → set angle constraints → verify elbow respects limits → create a 4-bone chain → switch to FABRIK → verify it solves
10. **Spring physics**: Add physics constraint to a socketed layer → set frequency=2, damping=0.3 → animate parent → play → verify child overshoots and settles → add gravity (270°, moderate strength) → verify element sags downward → add wind → verify sinusoidal sway → restart animation → verify spring state resets
11. **Squash & stretch**: Enable volume-preserve on an element → keyframe scale.y squash → verify scale.x automatically compensates → verify it works during animation playback
12. **Procedural modifiers**: Add sine modifier on position.y (0.5Hz, small amplitude) → play → verify smooth floating motion → add noise modifier on rotation → verify organic wobble → verify modifiers layer additively on top of keyframed values
13. **Look-at**: Add look-at constraint on an element → set target element → verify rotation follows target → set angle limits → verify clamping → enable spring smoothing → verify smooth tracking instead of snap → move target past angle limits → verify element stops at limit
14. **Lospec import**: Import a palette → verify it replaces the current one → verify existing elements remap by index → import a shorter palette → verify out-of-range indices fall back to transparent
15. **Skins**: Create a skin → override strokeColorIndex and fillColorIndex on several elements → switch between default and skin in the dropdown → verify canvas updates to show skin overrides → verify drawing modifies base geometry (shared) while rendering with skin → duplicate a skin → modify the duplicate → verify original is unchanged → delete a skin → verify undo restores it → export → verify separate atlas per skin and shared animation RON with skin manifest
16. **Export (runtime bone)**: Save sprite → check output directory for texture atlas + RON animation data → verify per-element part PNGs are packed correctly → verify RON contains keyframes, IK chains, physics params, skin manifest → test in a Bevy project with runtime evaluator + hot-reload → verify socketed layers and procedural animation work at 60 FPS → verify skin switching loads correct atlas
17. **Export (spritesheet, if implemented)**: Export a simple VFX sprite → verify atlas PNG + TextureAtlasLayout RON → verify configurable FPS → verify physics bakes correctly via sequential evaluation
18. **Autosave**: Make changes → wait 3 seconds → verify file saved automatically → switch tabs → verify save triggers → verify no "unsaved changes" dialogs
19. **Navigation**: Double-click sprite card → verify editor tab opens → open multiple sprites → verify tabs work → verify project overview stays as first tab → on project overview, verify sprites render animations live → switch animation sequence and skin via dropdowns → verify preview updates → drag sprites to compose them together
20. **Watcher**: Start watcher → modify and save a .sprite file externally → verify only that sprite re-exports (not all sprites)
21. **Undo + physics**: Change a spring parameter mid-animation → undo → verify playhead stays at current position (FK-only pose) → replay → verify simulation re-runs correctly from frame 0
