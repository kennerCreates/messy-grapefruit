# SVG Sprite Drawing & Animation Tool

## Context

Building a native Rust desktop app (eframe/egui) for creating animated SVG sprites for a 2D isometric Bevy game. The art style targets **high-resolution isometric line art** (similar to *They Are Billions*), not pixel art. The tool draws vector art using lines/curves with an indexed color palette, animates via pose-based keyframes with editable easing curves, and exports runtime bone animation data (RON) + texture atlases that Bevy hot-reloads.

**This is an artist-forward tool.** Every design and implementation decision should prioritize the artist's workflow, comfort, and creative flow. The UI should feel like a drawing tool, not a developer tool — visual, intuitive, and minimal friction. When in doubt, ask "would an artist find this natural?" Icons over text. Direct manipulation over forms. Presets over raw parameters. The tool should get out of the way and let the artist draw.

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
├── strokeTaper: bool                    // project-wide toggle (default true). Pointed-ends taper on all strokes
├── hatchPatterns: HatchPattern[]        // user-created fill patterns, shared per-project
│   └── HatchPattern { id, name,
│         layers: HatchLayer[] }         // multi-layer hatching (e.g., cross-hatch = two layers at different angles)
│       └── HatchLayer { angle: f32, spacing: f32, strokeWidth: f32, colorIndex: u8,
│             offset?: f32 }             // offset shifts the line set (for staggered patterns)
│       // Patterns are project-wide definitions — elements reference them by ID
│       // Import/export as standalone JSON for cross-project sharing
└── editorPreferences (theme, gridSize, gridMode, showDots, showLines)

Sprite (.sprite file)  // canvasWidth/canvasHeight = export pixel dimensions (1:1, no scale factor)
├── id, name, formatVersion, canvasWidth, canvasHeight, backgroundColorIndex (default 0/transparent)
├── layers: Layer[]
│   └── Layer { id, name, visible, locked, elements: Element[],
│         socket?: { parentElementId, parentVertexId },   // if set, layer follows this vertex
│         groupId?: string }                              // if set, layer belongs to this LayerGroup
├── layerGroups: LayerGroup[]
│   └── LayerGroup { id, name, collapsed: bool, visible, locked }
│       // Purely organizational — no effect on rendering order or sockets
│       // Visibility/lock cascade to all child layers when toggled
│       └── constraints?: LayerConstraints
│       └── Element = StrokeElement | IKTargetElement
│           StrokeElement { id, name?, type: "stroke", vertices: PathVertex[], closed, strokeWidth,
│               strokeColorIndex, fillColorIndex, position, rotation, scale, origin: Vec2,
│               taperOverride?: bool,    // per-element opt-out (null = use project default)
│               gradientFill?: GradientFill,   // overrides flat fillColorIndex when set
│               hatchFillId?: string,          // references a project HatchPattern by ID
│               hatchFlowCurve?: Vec2[] }      // control points for curving hatch lines within element
│           GradientFill { type: "linear"|"radial", colorIndexStart: u8, colorIndexEnd: u8,
│               angle?: f32,                   // linear: direction in degrees (default 0 = left→right)
│               center?: Vec2, radius?: f32 }  // radial: center + radius (normalized 0–1 within element bounds)
│           // gradientFill interpolates between two palette colors — stays within indexed color system
│           // hatchFlowCurve: bezier control points defining how hatch lines bend within this element
│           //   absent = straight lines at the pattern's angle; present = lines warp to follow the curve
│           IKTargetElement { id, name?, type: "ik-target", position: Vec2, ikChainId: string }
│               (lightweight — no vertices/strokes, renders as crosshair icon on canvas)
│               (position is world-space — not relative to the layer. Lives on tip layer for organization only)
│           └── PathVertex { id, pos: Vec2, cp1?: Vec2, cp2?: Vec2, manual_handles?: bool }
│               (cp1/cp2 = cubic bezier handles; absent = straight line)
│               (manual_handles = true when user has explicitly dragged handles; recompute_auto_curves preserves these)
│               (origin = user-defined pivot point for rotation/scale, snaps to grid)
│
│   LayerConstraints {
│     volumePreserve?: boolean,                          // scale_x = 1/scale_y
│     lookAt?: { preset?: string,                        // "snap"|"smooth"|"lazy" — sets defaults, then user tweaks
│       targetElementId, targetVertexId?,                // aim at element origin or specific vertex
│       restAngle, minAngle, maxAngle, mix, smooth?: { frequency, damping } }
│     physics?: { preset?: string,                       // "stiff"|"soft"|"hair-cape"|"tail"|"jiggle"|"heavy-bounce"
│       frequency, damping, mix,                         // spring follow (UI labels: bounciness, settle speed, strength)
│       gravity?: { angle, strength },                   // constant force (angle in degrees, 270 = down). UI label: weight
│       wind?: { strength, frequency } }                 // sinusoidal force. UI labels: sway amount, sway speed
│     procedural?: ProceduralModifier[]                  // additive oscillation
│   }
│   ProceduralModifier { preset?: string,                // "breathing"|"floating"|"flickering"|"wobble"|"pulsing"
│     property, waveform: "sine"|"noise", amplitude, frequency, phase, blend: "additive"|"multiplicative" }
│   // UI labels: amplitude→intensity, frequency→speed, phase→(advanced), blend→(advanced, default additive)
├── skins: Skin[]
│   └── Skin { id, name, overrides: SkinOverride[] }
│       └── SkinOverride { elementId, strokeColorIndex?, fillColorIndex?, strokeWidth?,
│             gradientFill?: GradientFill?, hatchFillId?: string? }
│           // Each override replaces visual properties on a specific element
│           // Omitted fields inherit from the base element
│           // The base sprite (no skin applied) is the implicit "default" skin
├── transitions: AnimationTransition[]
│   └── AnimationTransition { id, fromAnimationId: string, toAnimationId: string }
│       // Visual documentation of how animations connect in the game state machine
│       // No logic — just arrows between animation nodes for artist reference
│       // Exported in RON so Bevy can reference the intended transition map
└── animations: AnimationSequence[]
    └── AnimationSequence { id, name, duration, looping, poseKeyframes: PoseKeyframe[], ikChains: IKChain[],
          eventMarkers: EventMarker[] }
        └── EventMarker { id, time: f32, name: String }
            // Named markers at specific times — fire game events in Bevy at runtime
            // Just string labels + time positions, no logic in the editor
        // duration auto-extends when a pose keyframe is placed past the current end; also manually editable (e.g., to add trailing hold time)
        └── PoseKeyframe { id, time, easing: EasingCurve, elementPoses: ElementPose[], ikMixValues: [(chainId, mix)]  }
            // Captures the FULL state of all elements at a point in time — pose-to-pose animation
            // Easing curve controls the transition TO this pose from the previous one
            └── ElementPose { elementId, layerId, position, rotation, scale, visible,
                  strokeColorIndex, fillColorIndex, vertexPositions: [(vertexId, Vec2)],
                  gradientColorIndexStart?, gradientColorIndexEnd? }
                // Per-element snapshot — all animatable properties in one struct
                // Gradient color indices captured for animation (step interpolation, same as flat fills)
            └── EasingCurve { preset, controlPoints: [x1,y1,x2,y2] }
        └── IKChain { id, name, layerIds: string[],       // ordered root→tip socket chain
              targetElementId: string,                       // references an IKTargetElement (position captured in ElementPose)
              mix: number,                                   // 0=FK, 1=IK, stored per-pose in ikMixValues
              bendDirection: 1|-1,                           // sign flip for 2-bone
              solver: "two-bone"|"fabrik",                   // analytical or iterative
              angleConstraints?: { layerId, min, max }[] }   // per-joint angle limits (2-bone only initially)
```

**Pose-based animation**: Each `PoseKeyframe` snapshots the full sprite state (all element positions, rotations, scales, vertex positions, colors, visibility) at a point in time. The animation system interpolates between adjacent poses using the easing curve on each pose. This is simpler than per-property tracks — the artist poses the sprite and the system captures everything at once. With **auto-key mode** enabled, keyframes are created/updated automatically when the artist modifies the sprite while the playhead is on the timeline, eliminating the manual "Insert Pose" step.

**Animatable properties** (captured per-element in each pose): `position`, `rotation`, `scale`, `strokeColorIndex`, `fillColorIndex`, `gradientColorIndexStart`, `gradientColorIndexEnd`, `visible`, and all vertex positions (by stable vertex ID). Color indices (including gradient endpoints) and visibility are interpolated as integers (nearest/step). Continuous properties (position, rotation, scale, vertex positions) use the pose's easing curve.

*Vertex animation uses stable vertex IDs (not positional indices) so poses survive vertex insertion/deletion.*

**Visibility**: Each `ElementPose` has a `visible` boolean. Elements not present in a pose or marked invisible are excluded from the evaluation pipeline entirely — no IK, physics, or constraints until visible.

**Rest pose**: Frame 0 with no animation playing is the canonical rest/bind pose. All element positions, rotations, scales, and vertex positions at frame 0 define the default state. IK bone lengths are computed from the rest pose (distance from socket vertex to child layer's origin). The export pipeline uses the rest pose as the reference for default transforms and bone setup. Editing the sprite with the playhead at frame 0 and no sequence selected modifies the rest pose directly.

---

## Key Behaviors

### Viewport controls
- **Zoom**: Scroll wheel (centered on cursor)
- **Pan**: Middle-click drag
- **Right-click**: Cancel current tool action and return to the select tool. If the line tool is mid-stroke (vertices placed but not finished), right-click discards the in-progress stroke and switches to select. If already in select tool, right-click does nothing (reserved for future context menus if needed)
- **Canvas flip** (view only): Hotkey (`H`) mirrors the viewport horizontally without modifying any data. A classic artist trick to spot asymmetry and proportion errors. The flip is purely visual — coordinates, snapping, and export are unaffected. A subtle indicator in the status bar shows when the view is flipped
- **Zoom to selection**: Hotkey (`F`) zooms and pans the viewport to frame the currently selected element(s) with padding. If nothing is selected, frames all visible content. Standard navigation shortcut (Blender, Figma, Illustrator)

### Canvas state indicator
- A subtle color-coded border or highlight along the canvas edge indicates the current animation editing state at a glance:
  - **Blue**: Rest pose (no animation selected, or playhead at frame 0 with no sequence)
  - **Green**: Playhead is exactly on a keyframe (in-place editing)
  - **Orange**: Playhead is between keyframes (interpolated state)
  - **No border / default**: No animation sequence selected and not in rest pose mode
- Prevents the common mistake of accidentally editing the wrong keyframe or unknowingly modifying the rest pose when the artist meant to edit a specific animation pose
- The color is shown as a thin (2-3px) line along the top edge of the canvas, plus a small colored dot icon in the status bar matching the border color (blue/green/orange). No text label — the color alone communicates the state

### Sprite metrics bar
- A persistent info readout in the status bar showing stats for the current sprite: element count, vertex count, layer count, animation count, and estimated export atlas size (e.g., "512x256 px")
- Always visible when editing a sprite — updates in real-time as elements are added/removed
- Helps the artist gauge complexity at a glance — if vertex count climbs past ~500 on a single sprite, that signals over-detailing for a game asset
- Atlas size estimate is based on current export settings (layout, trim, padding, frame count) — recalculates on any change that affects it
- Purely informational, no interaction — each stat shown as a small icon (element, vertex, layer, animation, atlas) paired with its number. No text labels

### Hover highlight
- When the select tool is active, elements under the cursor display a **highlight outline** before the artist clicks — previewing what will be selected
- The highlight uses a distinct color (e.g., theme accent at 50% opacity) and follows the element's stroke path
- If multiple elements overlap under the cursor, the topmost visible, unlocked element is highlighted
- Works across layers (respects visibility and lock state)
- Eliminates guesswork in dense sprites where many elements overlap — the artist sees exactly what they'll grab before committing

### Selection stack popup
- When clicking in an area where **multiple elements overlap**, hold a modifier key (Alt+click) or use a quick popup to cycle through the overlapping elements
- The popup shows a small list of elements under the cursor, ordered top-to-bottom by render order. Each entry displays a **thumbnail preview** (miniature rendering of the element's stroke) alongside the element name, so the artist can visually identify which element they want
- Click an entry to select that element specifically, bypassing the default "topmost element" behavior
- Essential for complex sprites with 20+ overlapping body parts at joints — without this, selecting a buried element requires hiding/locking layers above it

### Reference image overlay
- Import a reference image (PNG/JPG) onto the canvas via toolbar button or drag-and-drop
- The reference image is **not exported** — it exists only as an editing aid
- Controls: **position** (drag to move), **opacity** slider (default 30%), **lock** toggle (prevent accidental selection/movement), **visibility** toggle
- Renders behind all layers (bottommost z-order). Not affected by palette or skin changes
- Multiple reference images supported per sprite (each independently positioned/toggled)
- Reference image paths are stored in the `.sprite` file (relative to project directory) but the image data is not embedded — keeps file sizes small
- Use case: tracing over concept art, matching proportions to an existing sprite, using a turnaround sheet for multi-angle consistency

### Import SVG paths
- Import an existing SVG file (from Inkscape, Illustrator, etc.) via File → Import SVG or drag-and-drop. Parses `<path>` elements and converts them into native `StrokeElement`s — fully editable after import
- **Import dialog** with a **scale modifier** (default 1.0): scales all imported path coordinates and stroke widths uniformly before placing on canvas. Essential for matching source SVG dimensions to the sprite's canvas size (e.g., an Inkscape SVG at 1000px wide imported into a 256px canvas needs ~0.25x scale)
- **Stroke width normalization**: imported stroke widths (after scaling) are snapped to the nearest project-standard width step (1, 2, 4, 8 px). Keeps imported art consistent with hand-drawn elements instead of introducing arbitrary fractional widths
- **Palette color matching**: SVG stroke/fill hex colors are mapped to the **nearest existing palette color** by perceptual color distance (CIELAB delta-E). No new colors are added to the palette — everything snaps to existing entries, maintaining art style consistency
- **Stroke taper**: imported open paths automatically inherit the project-wide taper setting (since taper is a rendering effect, not stored data). Imported art immediately gets the same pointed-ends style as hand-drawn strokes
- Each imported `<path>` becomes a separate `StrokeElement` with cubic bezier handles converted from the SVG path data. Grouped `<g>` elements are flattened (groups are not preserved — all paths land on a single new layer)
- Transforms on SVG elements (`translate`, `rotate`, `scale`, `matrix`) are baked into vertex positions during import
- Unsupported SVG features (gradients, filters, text, `<rect>`/`<circle>`/`<ellipse>` primitives) are silently skipped — only `<path>` data is imported. A toast notification lists skipped element counts if any
- The import dialog shows a preview of what will be imported (path outlines with matched palette colors) before confirming, so the artist can adjust scale and verify color matching

### Eyedropper tool
- Hotkey `I`, or hold `Alt` while in any drawing tool (line, fill) to temporarily activate
- Click an element on canvas to sample its **stroke color index** as the active color. Shift+click to sample the **fill color index** instead
- The sampled palette index becomes the active color for subsequent drawing/fill operations
- Works across layers (samples whatever is visually under the cursor, respecting layer visibility)
- Brief visual feedback: a small color swatch tooltip appears next to the cursor showing the sampled color

### Symmetry drawing mode
- Toggle a **mirror axis** on the canvas (vertical, horizontal, or both) via toolbar button or hotkey (`S`)
- When active, every vertex placed with the line tool automatically creates a mirrored counterpart on the opposite side of the axis. The mirrored vertices form a separate element on the same layer
- The mirror axis is a draggable guide line on canvas (default: vertical, centered on canvas width). Position snaps to grid
- Auto-curve control points are mirrored accordingly (cp1/cp2 flipped across the axis)
- Only affects **new vertices during line tool drawing** — existing geometry is not modified. Use layer mirror for post-hoc mirroring
- The mirrored element is a fully independent element (own ID, own vertices) — not a live-linked clone. After drawing, the two sides can be edited independently
- Visual feedback: the mirror axis renders as a subtle dashed line, and a ghost preview of the mirrored stroke follows the cursor in real-time
- Status bar shows a symmetry axis icon (vertical line, horizontal line, or cross) when active — no text
- Use case: drawing symmetrical faces, torsos, shields, helmets — draw one side, get the other free

### Stroke preview
- While the line tool is active and vertices have been placed, a **rubber band preview** shows what the next line segment will look like before the artist clicks
- The preview line follows the cursor in real-time, including auto-curve shaping (Catmull-Rom preview) so the artist can see exactly how the curve will bend
- Rendered as a semi-transparent stroke in the active color, with a dotted or dashed style to distinguish it from committed segments
- When approaching a merge target vertex, the preview snaps to it and shows the merge highlight

### Stroke taper (pointed ends)
- A **project-wide toggle** (default on) that automatically tapers stroke width to zero at both endpoints of every open path, with full width at the midpoint. Gives all line art a hand-inked quality — thick confident strokes that taper to fine points at their tips
- The taper follows a smooth curve along the path's normalized length (0.0 at start → 1.0 at end). At any point `t` along the path, the rendered width is `strokeWidth * (1 - (2t - 1)²)` — a parabolic falloff that peaks at the center and reaches zero at both ends
- **Closed paths are excluded**: taper only applies to open paths. Closed shapes (circles, outlines) render at uniform `strokeWidth` since they have no endpoints
- The toggle lives on the **Project** (like the palette), so changing it instantly updates all strokes across all sprites — ensuring a consistent art style
- Individual elements can **opt out** via a per-element `taperOverride` toggle in the select tool sidebar. This is for cases where uniform width is intentional (e.g., a perfectly even border line). Elements without an override follow the project default
- The taper is purely a rendering effect — it does not modify `strokeWidth` or vertex data. Toggling it off restores uniform strokes everywhere with no data loss
- SVG export and rasterization both respect taper (SVG uses variable-width stroke paths)

### Element isolation (solo mode)
- Click a **solo button** (eye icon variant) on a layer in the layer panel to isolate it
- All other layers dim to ~15% opacity, making the soloed layer visually prominent
- Multiple layers can be soloed simultaneously (shift+click solo buttons)
- Solo mode is purely visual — it does not affect selection, locking, or export
- Useful for complex sprites with 20+ body-part layers where overlapping geometry makes it hard to see what you're editing
- A "clear solo" button in the layer panel header exits solo mode for all layers at once

### Snap to vertices
- When dragging a vertex near an existing vertex on **any visible, unlocked layer**, a magnetic snap indicator appears and the vertex snaps to the target position
- This supplements grid snapping — grid snap applies first, then vertex snap overrides if a nearby vertex is within a threshold (configurable, default ~8 screen pixels)
- Visual feedback: the target vertex highlights with a ring/diamond indicator, same style as the merge preview but distinguished by color (e.g., blue for snap, green for merge)
- Essential for aligning body parts at joints across layers, ensuring socket points line up, and maintaining symmetry
- Can be toggled on/off via a toolbar toggle or hotkey

### Recent colors
- A small bar of the **last 8 used palette colors** displayed at the top of the palette panel (above the full swatch grid)
- Updates automatically as the artist draws or fills — most recently used color appears first
- Click a recent color to set it as active, same as clicking a swatch in the full palette
- Persisted per-session (resets when the editor closes). Not saved to project file
- Saves scrolling through a 256-color palette to re-find colors the artist just used

### Animation UX

**Auto-key mode**: A toggle button on the timeline toolbar. When enabled, any modification to element properties (position, rotation, scale, vertex positions, colors, visibility) while an animation sequence is selected automatically creates a new `PoseKeyframe` at the current playhead time, or updates the existing keyframe if the playhead is exactly on one. This eliminates the manual "Insert Pose" step — the artist just poses and moves the playhead. Auto-key is off by default to prevent accidental keyframing while drawing. Visual indicator: the timeline background or playhead turns a warm color (e.g., red tint) when auto-key is active.

**In-place pose editing**: When the playhead lands exactly on an existing keyframe, the canvas displays that pose's stored state. Any modifications update the existing keyframe directly rather than creating a new one (regardless of auto-key mode). The keyframe marker on the timeline changes color (e.g., filled vs. outlined) to indicate "editing existing pose" vs. "no keyframe at this time." This makes iterating on a pose natural — scrub to the keyframe, tweak, done.

**Start from interpolated state**: When the playhead is between two keyframes and the artist begins editing (or inserts a pose manually), the canvas starts from the interpolated pose at that time — not the rest pose or the last keyframe. The new keyframe captures this interpolated state plus whatever adjustments the artist makes. This means the artist only needs to make small delta adjustments rather than re-posing from scratch.

**Pose copy / paste / mirror**: Right-click a keyframe marker on the timeline to access:
- **Copy pose**: Copies the pose data to an internal clipboard
- **Paste pose**: Inserts the copied pose at the current playhead time (creates a new keyframe with the copied element states)
- **Mirror pose**: Creates a mirrored copy of a pose — flips all element positions horizontally around the sprite's canvas center, negates rotations, and swaps left/right vertex positions. Essential for walk cycles (copy left-leg-forward pose, mirror to get right-leg-forward, halving the work)
- **Duplicate pose**: Shortcut to copy + paste at a different time

**Pose thumbnails**: Each keyframe marker on the timeline displays a small thumbnail (approximately 32×32 pixels) showing a miniature rendering of the sprite in that pose. Thumbnails are cached and regenerated when the pose is modified. This gives instant visual context — the artist can see "crouching, jumping, landing" at a glance instead of anonymous tick marks.

**Transition duration handles**: The space between two adjacent keyframe markers on the timeline is draggable. Dragging the gap wider increases the time between poses (slower transition); dragging it narrower decreases the time (faster transition). This adjusts the `time` values of downstream keyframes. More intuitive than editing millisecond values manually. Holding Shift while dragging moves only the right keyframe (adjusts one transition without affecting subsequent timing).

**Onion skinning — keyframe mode**: In addition to the standard "N frames before/after" onion skinning, a **keyframe mode** toggle shows ghost overlays of the previous and next *keyframe poses* specifically (not time-adjacent frames). This is more useful when posing — the artist sees where the sprite came from and where it's going, regardless of how far apart the keyframes are in time. Both modes can be active simultaneously. **Configurable onion skin colors**: The before/after ghost colors are user-configurable (defaults: red for previous, green for next). Accessible via a small settings button on the onion skinning toggle. Helpful for sprites with red/green color schemes where default ghost colors would blend in.

**Keyframed vs. interpolated visual indicators**: On the timeline, keyframe markers display as **filled diamonds** when the playhead is on them (editing state) and **outlined diamonds** when the playhead is elsewhere. Additionally, when the playhead is between keyframes, a small interpolation indicator (dotted diamond) appears at the playhead position to reinforce that the current canvas state is derived, not stored. This visual distinction prevents the common mistake of thinking an interpolated frame is a keyframe and wondering why changes "don't stick" (when auto-key is off).

**Inline easing curve editing**: Clicking the timeline segment between two keyframes opens a small popup anchored to that segment showing the easing curve. The popup displays a standard cubic bezier curve editor with draggable control points, plus preset buttons (linear, ease-in, ease-out, ease-in-out, bounce, elastic). Changes apply immediately with live preview on canvas. This is faster than opening a separate dialog or editing numeric control points — the artist stays in the timeline flow.

**Animation event markers**: Named markers placed on the timeline at specific times that represent game events (e.g., "footstep", "spawn_projectile", "play_sound", "hitbox_active"). Event markers are just string labels at a time position — no logic executes in the editor. They appear as small labeled flags above the timeline's time axis, color-coded differently from pose keyframe diamonds. Right-click the timeline ruler to add/rename/delete event markers. Markers are draggable along the time axis to adjust timing. Exported in the RON animation data so the Bevy runtime can react to them (e.g., play a sound effect when the "footstep" marker time is reached during playback). Without event markers, the game would have to hardcode frame numbers for gameplay-relevant moments, which breaks whenever animation timing changes.

**Animation templates**: Pre-built timing templates for common animation patterns. Selecting a template creates keyframes with appropriate timing (the artist then adjusts the poses):
- **Idle / breathing**: 2 keyframes, slow ease-in-out, looping (~2s cycle)
- **Walk cycle**: 4 keyframes at equal spacing, looping (~0.8s cycle). Contact-passing-contact-passing pattern
- **Attack**: 3 keyframes — anticipation (short), contact (snap), follow-through (ease-out). Non-looping
- **Jump**: 4 keyframes — crouch (anticipation), launch, apex (hang time with slow ease), land. Non-looping
- Templates set only timing and easing; all poses start as copies of the current sprite state. The artist modifies each pose from there
- Accessible via a dropdown or menu in the timeline panel

### Curve handles & straight/curve toggle
- **Auto-curve is the default**: When placing vertices, control points are auto-generated using Catmull-Rom → cubic bezier conversion. Endpoints use duplicated-endpoint phantom points (zero curvature / straight tangent at path ends)
- **Editable handles**: Selected curved vertices display draggable cp1/cp2 control point handles on the canvas. Dragging a handle updates the bezier curve in real-time
- **Straight/curve hotkey toggle**: A single hotkey (e.g., `C`) toggles the line tool between curve mode and straight mode while drawing. Indicated visually in toolbar/status bar. Can also toggle per-vertex after placement by selecting a vertex and pressing the hotkey

### Grid
- Grid size is **manually set** by the artist (stored in `editorPreferences.gridSize`). Available sizes: 1, 2, 4, 8, 16, 32, 64 px. Changeable via sidebar (Settings mode) or a toolbar dropdown
- Grid stays at the chosen size regardless of zoom level — zooming in reveals the same grid, just larger on screen. No automatic density changes
- **Snapping is always active** and always uses the **isometric diamond lattice** with basis vectors `u=(2gs, gs)`, `v=(2gs, -gs)` regardless of the `gridMode` setting. This ensures all vertices land on lattice points suitable for isometric art
- **Grid dots** (`showDots`): toggleable on/off. Dots render on a **staggered isometric diamond lattice** — even rows at `x=0, ±4gs, ±8gs...`, odd rows offset at `x=±2gs, ±6gs...`. At extreme zoom-out where dots would overlap, dots are hidden (snapping still works)
- **Grid lines** (`showLines`): toggleable on/off, independent of dots. Two line modes (`gridMode`): **straight** (square grid lines) and **isometric** (2:1 ratio diagonal lines at ±0.5 slope). Lines use a **lower contrast** color than dots — closer to the canvas background color. In isometric mode, the diagonal lines pass through the dot positions
- Dots and lines can be on/off independently — both on, both off, or either one alone. Snapping works in all combinations

### Indexed color palette (per-project)
- The palette lives on the **Project**, not on individual sprites — all sprites in a project share the same palette
- Elements store palette index, not color values
- Changing a palette color instantly updates all elements across all sprites referencing that index (renderer looks up color at render time)
- Index 0 is always transparent/none
- Color index animation uses **nearest/step** interpolation within pose transitions (integer values snap rather than blend smoothly)
- The palette is passed to the sprite editor when opening a sprite, and saved with the project file
- **Lospec import replaces** the current palette. Existing color indices remap to the same index in the new palette (index 3 stays index 3). If the new palette is shorter, elements referencing out-of-range indices fall back to index 0 (transparent)
- **Color ramp finder**: Select a base color from the palette, then the tool scans existing palette entries to find related shades (by hue proximity and lightness variation). Presents the best 3–5 matching colors as a sorted ramp (highlight → base → shadow). Click any color in the ramp to set it as active. No new colors are generated — this is purely a palette navigation aid that groups existing colors by visual relationship. Useful when working with a large imported palette where the artist hasn't memorized which indices are light/dark variants of the same hue

### Eraser tool
Two modes, determined by what the artist clicks:
- **Click a vertex**: Delete the vertex and all line segments connected to it. If the path is split into disconnected parts, they become separate elements. Both resulting elements inherit `position`, `rotation`, `scale`, `origin`, and color indices from the original
- **Click a line segment**: Delete just the segment between two vertices. Vertices on either end are kept if they still connect to at least one other segment. Vertices that would become islands (no remaining connections) are automatically deleted. If the path is split into disconnected parts, they become separate elements (same inheritance rules as vertex deletion)
- **Pose data on split**: Existing pose keyframes are updated — the original element's pose entry is duplicated for both resulting elements (same position/rotation/scale/colors), and vertex positions are split according to which element each vertex belongs to. Vertex positions referencing deleted vertices are dropped

### Layer operations
- Layers are groups containing multiple elements. Elements render in creation order within a layer; layers render bottom-to-top
- All layer actions are **icon buttons** in the layer panel header (with tooltips):
  - **Add** (plus icon) — new layer
  - **Remove** (trash icon) — delete layer
  - **Duplicate** (copy icon) — deep-copies all elements with new IDs. Pose keyframe entries, socket references, and constraints are not copied for the new elements
  - **Mirror** (flip icon) — flip all elements horizontally or vertically around the bounding box center of the layer's elements. Flips vertex positions and control points. Useful for creating symmetrical body parts — e.g., duplicate left arm, mirror to make right arm
  - **Combine** (merge icon) — merge two layers into one. If either layer is socketed, the combined layer keeps the socket of the *top* layer. If only the bottom layer was socketed, a warning dialog is shown before proceeding (socket will be dropped). If either layer is a socket parent for other layers, those child references update to point to the combined layer
  - **Move** — drag to reorder (no button needed)
- Per-layer icon toggles: **visibility** (eye icon), **lock** (padlock icon)

### Layer groups (folders)
- Layers can be organized into collapsible **groups** (folders) in the layer panel — purely organizational, no effect on rendering order or socket behavior
- **Create group**: button in layer panel header, or drag a layer onto another layer to form a group
- **Collapse/expand**: click the group's disclosure triangle to hide/show its child layers. Collapsed groups show a layer count badge (e.g., "Left Arm (5)")
- **Visibility/lock cascade**: toggling a group's visibility or lock state applies to all child layers within the group. Individual layers can still override within the group
- **Drag in/out**: layers can be dragged into a group, out of a group, or reordered within a group. Groups can be reordered among other groups and ungrouped layers
- **Nesting**: groups cannot contain other groups (single level only) — keeps the UI simple and avoids deep hierarchy complexity
- **Delete group**: removes the group folder but keeps its layers (they become ungrouped). Option to delete group + all child layers via Shift+Delete
- Use case: a character with 20+ layers (head, eyes, torso, left arm upper/lower/hand, right arm upper/lower/hand, left leg upper/lower/foot, etc.) — group them by body part to keep the layer panel manageable

### Fill tool
- Click a **closed** element to set its `fillColorIndex` to the active palette color
- Click **empty canvas or inside an open path** to set the sprite's `backgroundColorIndex` (like Paint's bucket fill)

### Gradient fill
- Available on any **closed** element as an alternative to flat fill. Set via the sidebar when an element is selected — three icon toggles for fill mode: solid square (flat), gradient bar (linear), radial circle (radial)
- **Linear gradient**: interpolates between two palette color indices along a direction. The artist picks start color, end color, and angle (0–360°). Rendered as smooth dithered steps between the two palette colors
- **Radial gradient**: interpolates from center color outward to edge color. Adjustable center point (drag handle on canvas) and radius
- Stays within the indexed color system — both endpoints must be existing palette colors. The gradient generates intermediate steps by blending between the two palette entries at render time
- Gradients are animatable — `colorIndexStart` and `colorIndexEnd` are captured in `ElementPose`, so gradient colors can change between keyframes (step interpolation, same as flat fills)
- Exported as part of the element data in RON. The Bevy runtime shader handles gradient rendering

### Hatch fill patterns
- **Project-level pattern library**: hatch patterns are defined at the project level (stored in `.spriteproj`) and referenced by ID on individual elements. Each pattern has a name and one or more **hatch layers** — each layer defines an angle, spacing, stroke width, and color index. A single-layer pattern produces parallel lines; a two-layer pattern at perpendicular angles produces cross-hatching
- **Pattern editor**: accessible from the sidebar (Settings mode → Hatch Patterns tab). The **live preview swatch is primary** — it dominates the editor, showing the pattern at current settings. Parameter sliders (angle, spacing, width, color per layer) are secondary, below the preview. Create, rename, duplicate, delete patterns via icon buttons. Add/remove hatch layers within a pattern
- **Applying a pattern**: select a closed element → in the sidebar fill options, select the hatch icon toggle (fourth fill mode alongside flat/linear/radial) → pick a pattern from a visual swatch grid (not a text dropdown). The hatch lines are generated to fill the element's closed path boundary, clipped to the shape
- **Flow curves**: by default, hatch lines are straight at the pattern's defined angle. The artist can add a **flow curve** (a bezier guide path) to an element's hatch fill — the hatch lines then warp to follow the curve, creating effects like wood grain that follows a plank's shape or fabric folds that follow drapery. The flow curve is editable directly on canvas (drag control points) while the hatch fill is selected
- **Cross-project sharing**: patterns can be exported as standalone `.hatchpatterns` JSON files and imported into other projects. Import merges with existing patterns (skip duplicates by name)
- Hatch lines use the pattern's color indices from the project palette, so they stay consistent with the art style
- Hatch fills are rendered at export time — the lines are generated as SVG strokes clipped to the element path, then rasterized. No special Bevy runtime support needed (baked into the texture)

### Select tool
- Click to select, shift-click for multi-select, drag for marquee selection, Ctrl+A to select all elements on unlocked layers
- Ctrl+C/V for copy/paste, Delete key to remove
- Drag to move, handles for scale/rotate (pivot = element's user-defined origin, snaps to grid)
- **Copy/paste**: Paste creates a new layer containing copies of the selected elements. All pasted elements get new IDs. Pose keyframe entries, socket references, and layer constraints are not copied for pasted elements (constraints reference other elements/layers by ID and would break). Pasted layer is positioned with a small offset (+10, +10) from the original. **Cross-sprite paste**: Elements are serialized to the system clipboard as JSON, so copy/paste works across sprite tabs. Color indices reference the shared project palette, so colors stay consistent

### Palette constraints
- Max 256 colors. Index 0 = transparent/none
- When the limit is reached, show a toast notification ("Palette full — 256 color maximum")
- Sprites require project context to open (no standalone palette)

### Isometric grid mode
- 2:1 pixel ratio (26.57°), standard isometric — selected via `gridMode` toggle on grid lines
- Grid dots remain on the straight square grid — isometric mode only affects grid lines (diagonal lines pass through the dot positions)
- Snapping follows iso-grid intersections

### Layer sockets (transform parenting)
- A layer can be **socketed** to a vertex on any element in another layer. The socketed layer inherits the **position and rotation** of the parent vertex (scale stays independent)
- Any existing vertex can serve as a socket point — no special vertex type needed. Attach via the layer panel or a context menu
- Socket chains can be unlimited depth (arm → hand → weapon → gem). The renderer walks the chain root-to-leaf, accumulating position + rotation at each level
- Circular socket references are rejected at assignment time
- When the parent vertex is animated, the socketed layer follows automatically — no need to duplicate keyframes
- Socketed layers still have their own local position/rotation/scale (applied as offset relative to the parent vertex)
- Deleting a socket parent vertex shows a warning and detaches any socketed child layers (they snap to their current world-space position)
- **Socket visibility on canvas**: When a layer is selected that participates in a socket chain, faint dashed connection lines are drawn between parent vertices and child layer origins, showing the hierarchy visually. The lines use a subtle color (theme-aware, e.g., muted blue) and are only visible when relevant layers are selected — they don't clutter the canvas during normal drawing. This makes the parent-child relationship immediately obvious without having to read the layer panel

### Procedural Animation

**Evaluation order** (per frame, must be stepped sequentially from frame 0 due to stateful physics):
1. Evaluate FK from pose keyframes (interpolate between adjacent poses using easing curves)
2. Initial socket chain walk: compute world-space positions for all joints (needed by IK solver)
3. Solve IK chains (blended with FK via per-chain `mix`). IK bone length = distance from socket vertex to child layer's origin
4. Apply constraints: look-at (atan2 + angle limits + optional spring smoothing), volume preservation (scale_x = 1/scale_y)
5. Apply procedural modifiers: additive/multiplicative sine/noise
6. Apply physics simulation: spring dynamics chase the post-modifier values as targets (semi-implicit Euler, world space). Convert result back to local space. Gravity/wind operate in world space
7. Final socket chain walk: root-to-leaf, accumulating position + rotation with all modifications applied

**Constraint UX principles** (applies to all constraint types below):
- **Behavior presets**: Every constraint type has a preset dropdown that fills in all parameters with tested defaults. The artist picks a named behavior (e.g., "Hair/Cape sway"), sees it work, then optionally tweaks individual parameters. Presets are the primary workflow — raw parameter editing is secondary
- **Artist-friendly parameter names**: UI labels use intuitive names instead of physics/signal terms. Internal data model retains technical names for export compatibility. Mapping: frequency→Bounciness, damping→Settle speed, mix→Strength, gravity strength→Weight, wind strength→Sway amount, wind frequency→Sway speed, amplitude→Intensity, frequency(procedural)→Speed, waveform sine→Smooth, waveform noise→Noisy
- **Primary / Advanced split**: Each constraint shows only 2–3 essential sliders by default. A chevron disclosure icon toggles the full parameter set. Reduces visual noise and decision fatigue
- **Live preview while adjusting**: When the artist drags a constraint slider, the canvas runs a mini simulation loop in real-time — no need to press play. Immediate visual feedback makes parameter-tweaking intuitive even without understanding the underlying math. The mini simulation loops a short cycle (~2s) centered on the current playhead time
- **"Try it" mini-preview**: A small play icon button next to each constraint runs a 2-second isolated loop showing only that constraint's effect. No text label — just the play icon. Faster than playing the full animation and easier to evaluate one effect at a time
- **Quick-add buttons**: The layer panel and sidebar offer icon buttons for common effects — each effect has a distinct icon: Breathing (lungs/wave), Sway (wind/curve), Jiggle (vibration), Eye Track (eye), Tail Follow (arc), Bounce (spring). Tooltips show the effect name. Each adds the appropriate constraint type with a good preset — no need to understand that "Breathing" is a procedural sine modifier on scale.y

**Inverse Kinematics (IK)**
- **2-bone analytical solver**: Law of cosines. Covers arms/legs. Exact, no iteration. Bend direction is a +1/−1 sign flip on the offset angle
- **FABRIK solver**: For chains longer than 2 (tails, tentacles, spines). Forward-backward reaching, 3–10 iterations. Add tiny perturbation to avoid collinear deadlock
- **IK target**: A lightweight canvas element (position + crosshair icon, no vertices/strokes). Draggable on canvas, position captured in each pose's `ElementPose`. One target element per IK chain, lives on the chain's tip layer
- **FK/IK mix**: A 0–1 parameter per chain, stored per-pose in `ikMixValues`. At 0 = pure FK, at 1 = pure IK. Animate the mix across poses to smoothly transition mid-timeline (Spine-style)
- **Angle constraints**: Per-joint min/max angle relative to parent bone. Start with 2-bone only; skip for FABRIK initially
- **Bone length**: Distance from the socket vertex (on parent element) to the child layer's origin point. Computed from the rest pose
- IK chains are defined over sequences of socketed layers — the socket chain is the bone hierarchy

**Spring / Jiggle Physics**
- Per-layer opt-in constraint. The spring chases the layer's keyframed+IK+constraint-solved position as its target
- **Presets** (primary workflow):
  - *Stiff follow*: high frequency, high damping — snappy with minimal overshoot
  - *Soft follow*: medium frequency, low damping — gentle lag behind parent
  - *Hair/Cape sway*: low frequency + wind enabled — flowing secondary motion
  - *Tail drag*: low frequency, very low damping — heavy trailing follow
  - *Jiggle*: high frequency, medium damping — quick wobble that settles fast
  - *Heavy bounce*: low frequency, low damping + gravity — weighty overshoot
- **Primary sliders** (always visible): Bounciness (frequency, 0.1–10 Hz), Settle speed (damping, 0–2), Strength (mix, 0–1)
- **Advanced sliders** (toggle to reveal): Weight (gravity strength), Weight direction (gravity angle, default 270°=down), Sway amount (wind strength), Sway speed (wind frequency)
- Integration: semi-implicit Euler (`velocity += force * dt; position += velocity * dt`). Simulates in **world space** (so gravity/wind directions are absolute), result converted back to local space for the socket chain
- Spring state resets when animation restarts (snap to FK pose)

**Squash & Stretch**
- Per-layer `volumePreserve` toggle. When enabled, `scale_x = 1 / scale_y` is enforced automatically
- Works with keyframed scale, IK, and physics — applied as a post-constraint fixup
- Pivot is the element's `origin` point

**Procedural Modifiers**
- Per-layer list of additive oscillations on any animatable property
- **Presets** (primary workflow — one-click via quick-add buttons or preset dropdown):
  - *Breathing*: sine on scale.y, ~0.25 Hz, small amplitude, looping
  - *Floating / Hover*: sine on position.y, ~0.5 Hz, medium amplitude
  - *Flickering*: noise on rotation, high frequency, small amplitude
  - *Wobble*: noise on rotation, low frequency, medium amplitude
  - *Pulsing*: sine on scale (uniform x+y), ~1 Hz, small amplitude
- **Primary sliders** (always visible): Effect preset, Intensity (amplitude), Speed (frequency)
- **Advanced sliders** (toggle to reveal): Type (Smooth/Noisy — maps to sine/noise waveform), Phase offset (degrees), Blend mode (additive/multiplicative, default additive)
- Applied **before** physics, so spring dynamics can smooth procedural oscillation into organic secondary motion

**Look-At Constraint**
- Per-layer constraint. Layer rotates to face a target element (or a specific vertex on a target element)
- **Presets**:
  - *Snap tracking*: instant rotation to face target, no smoothing
  - *Smooth tracking*: spring smoothing with moderate frequency/damping
  - *Lazy follow*: spring smoothing with low frequency, high damping — slow, heavy tracking
- **Primary controls** (always visible): Target (element picker), Smoothness (maps to spring frequency/damping — single slider from 0=snap to 1=very lazy)
- **Advanced controls** (toggle to reveal): Rest angle, Min/Max angle limits, Strength (mix, 0–1), Spring frequency, Spring damping (overrides Smoothness slider for fine control)
- Handles angle wrapping at ±π (shortest angular difference)
- Good for: eyes tracking a point, turrets, head turns

### Keyboard shortcuts in tooltips & searchable overlay
- Every button and tool in the UI displays its keyboard shortcut in the tooltip (e.g., "Line Tool (L)", "Undo (Ctrl+Z)")
- A **searchable shortcut overlay** activated by pressing `?` shows all available shortcuts in a filterable list. The artist can type to filter (e.g., "zoom" shows zoom-related shortcuts). The overlay closes on Escape or clicking outside
- Shortcuts are grouped by category: Tools, Viewport, Animation, Layers, File
- This replaces the need for a separate "keyboard shortcuts" documentation page — the shortcuts are always discoverable in-app

### First-time contextual hints
- On first launch (or when a new feature is first encountered), small non-blocking hint bubbles appear near relevant UI elements with a brief explanation
- Examples: "Tip: Hold Alt to temporarily activate the eyedropper", "Tip: Right-click to cancel the current tool", "Tip: Press ? to see all keyboard shortcuts"
- Hints dismiss on click, and a "Don't show again" option disables all hints permanently
- Hints are stored in editor preferences (not project file) so they only appear once per user, not once per project
- Maximum 1 hint visible at a time — they never stack or overlap

### Skins (visual variants)
- A sprite can have multiple **skins** — named sets of visual overrides applied on top of the base element properties
- Each skin contains overrides per element: replacement `strokeColorIndex`, `fillColorIndex`, and/or `strokeWidth`. Omitted fields inherit from the base element
- The base sprite with no skin applied is the implicit "default" skin — it doesn't appear in the skins list
- **Bone structure, vertices, animations, sockets, IK chains, and constraints are shared across all skins** — only visual properties differ. Animate once, swap appearance
- **Skin selector** in the editor toolbar as a **thumbnail strip** — each skin shown as a small swatch/preview of its visual appearance, not a text dropdown. Click a thumbnail to switch skins
- When a skin is active in the editor, drawing/editing changes modify the **base** element (shared geometry), but the canvas renders with the skin's visual overrides so you can see the result
- **Export**: each skin produces its own texture atlas (different part PNGs), but all skins share the same animation RON data. The exported RON includes a skin manifest listing available skins and their atlas references
- Use case: same walk/attack/idle animations reused across soldier variants, enemy tiers, or equipment loadouts with different color schemes

### Undo/Redo
- Single shared stack for all mutations (drawing + animation)

### Autosave
- Debounced save 3 seconds after last change, plus save on tab switch and app blur
- No "unsaved changes" dialogs — undo stack handles mistakes
- **First save**: Creating a new project prompts for a save directory (`rfd` file dialog). New sprites are saved as `.sprite` files relative to the project directory. Autosave only activates once a file path is established
- **Crash recovery**: On each autosave, a `.sprite.recovery` file is written alongside the main `.sprite` file. The recovery file is a complete copy of the current state. On next launch, if a recovery file is newer than its corresponding `.sprite` file (indicating a crash between autosave and clean shutdown), the editor offers to restore from the recovery file. Recovery files are deleted on clean save/exit. This protects against data loss from crashes during editing sessions

### Navigation
- **Project overview** is always the first tab
- **Double-click** a sprite card to open it in a new editor tab
- Multiple sprites can be open simultaneously as tabs

### New Sprite dialog
- Canvas size presets as **clickable visual squares** at relative sizes (64x64 small, 128x128 medium, 256x256 large, 512x512 extra-large) with pixel dimensions shown below each. The artist sees the proportions at a glance
- Freeform width/height input for custom sizes
- Name field

### Game-resolution preview
- A small **floating preview window** that renders the sprite at its actual export resolution (1:1 pixel size, `canvasWidth` x `canvasHeight`), updated in real-time as the artist draws or animates
- Toggle via toolbar button or hotkey (`P`). The window is draggable and resizable, but always renders at native pixel size (no scaling) — if the window is larger than the sprite, the extra space is transparent/checkerboard
- Shows the sprite with all rendering effects applied: stroke taper, fills, active skin, current animation frame (including physics/procedural during playback)
- Stays in sync with the main canvas — any edit, tool change, or playhead scrub immediately updates the preview
- Use case: when zoomed in to 400% editing fine vertex positions or curve handles, the preview shows how the sprite actually looks at game size. Prevents over-detailing (adding complexity invisible at game resolution) and under-detailing (missing gaps visible only at 1:1)
- The preview window is editor-only state — not saved to the project file

### Canvas boundary
- A dashed rectangle on the canvas shows the export area (`canvasWidth` x `canvasHeight`). Art outside this boundary is preserved but excluded from spritesheet export. Bone export rasterizes each element at its own bounding box (canvas boundary is visual reference only)
- Boundary always visible regardless of zoom, rendered in a subtle contrasting color (theme-aware)

### Canvas resize
- Change `canvasWidth`/`canvasHeight` after creation via sprite settings
- Existing art stays at its current position (no scaling or repositioning)

---

## Workspace Structure

**Current state (Phase 2 complete):**

```
messy-grapefruit/
├── Cargo.toml
├── CLAUDE.md
├── plan.md
├── assets/icons/           (custom icon PNGs for toolbar/sidebar)
├── src/
│   ├── main.rs              (210 lines — App struct, eframe entry, action dispatch, keyboard undo/redo)
│   ├── action.rs            (11 lines — AppAction enum for canvas→app communication)
│   ├── clipboard.rs         (107 lines — copy/paste/cut with system clipboard + JSON serialization)
│   ├── model/
│   │   ├── mod.rs
│   │   ├── vec2.rs          (241 lines — Vec2 type + ops + conversions + tests)
│   │   ├── project.rs       (187 lines — Project, Palette, PaletteColor, EditorPreferences)
│   │   └── sprite.rs        (199 lines — Sprite, Layer, StrokeElement, PathVertex + manual_handles)
│   ├── state/
│   │   ├── mod.rs
│   │   ├── editor.rs        (293 lines — EditorState, ViewportState, SelectionState, SelectDragKind, VertexHover)
│   │   └── history.rs       (155 lines — snapshot undo/redo with drag coalescing)
│   ├── io.rs                (55 lines — save/load sprite JSON via rfd)
│   ├── math.rs              (465 lines — Catmull-Rom, bezier eval/split/flatten, fillet arcs, auto-curves, min radius enforcement)
│   ├── theme.rs             (188 lines — dark/light theme colors + apply + input styling)
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── icons.rs         (100 lines — icon asset loaders via include_image!, property icons)
│   │   ├── canvas.rs        (1118 lines — select tool orchestrator with 7 sub-functions, vertex editing, line tool, zoom-to-fit)
│   │   ├── canvas_input.rs  (257 lines — viewport input, line tool input, hotkeys)
│   │   ├── canvas_render.rs (540 lines — element rendering, highlights, previews, boundary, vertex dots, CP handles)
│   │   ├── grid.rs          (180 lines — dot/line rendering, straight + isometric)
│   │   ├── toolbar.rs       (214 lines — file ops, tools, grid controls, view, theme)
│   │   ├── sidebar.rs       (622 lines — context-sensitive tool options, element properties, layer list, palette swatches)
│   │   └── status_bar.rs    (40 lines — sprite metrics, flip indicator, grid mode)
│   └── engine/
│       ├── mod.rs
│       ├── snap.rs           (55 lines — grid snapping to isometric diamond lattice)
│       ├── hit_test.rs       (241 lines — point-to-stroke distance, vertex/handle hit testing in screen space)
│       ├── transform.rs      (258 lines — element transforms, world↔local conversions, selection bounds, recompute curves)
│       └── merge.rs          (117 lines — auto-merge coincident vertices at endpoints)
└── .gitignore
```

**Total: ~5,900 lines (Phase 2). Target workspace for all phases:**

```
src/
│   ├── state/
│   │   └── project.rs       (ProjectState, OpenSprite, tab management)    [Phase 15]
│   ├── ui/
│   │   ├── timeline.rs      (animation timeline, pose keyframes, playhead) [Phase 8]
│   │   ├── export_dialog.rs (export preview dialog with atlas image)       [Phase 14]
│   │   ├── new_sprite_dialog.rs (new sprite creation dialog)               [Phase 15]
│   │   └── project_overview.rs  (project dashboard with previews)          [Phase 15]
│   ├── engine/
│   │   ├── animation.rs     (pose interpolation, FK evaluation)            [Phase 8]
│   │   ├── socket.rs        (socket chain transforms, cycle detection)     [Phase 10]
│   │   ├── ik.rs            (2-bone analytical + FABRIK solvers)           [Phase 12]
│   │   ├── physics.rs       (spring simulation, gravity, wind)             [Phase 13]
│   │   ├── constraints.rs   (look-at, volume preserve, procedural)         [Phase 13]
│   │   └── hatch.rs         (hatch pattern generation, flow curves)        [Phase 6]
│   └── export/
│       ├── svg_gen.rs        (Sprite + time → SVG string)                  [Phase 7]
│       ├── rasterize.rs      (SVG → PNG via resvg)                         [Phase 7]
│       ├── bone_export.rs    (element → part PNGs + animation RON)         [Phase 14]
│       ├── ron_meta.rs       (Bevy-compatible RON metadata)                [Phase 14]
│       ├── spritesheet.rs    (frame atlas packing)                         [Phase 14]
│       └── watcher.rs        (file watcher for auto-export on save)        [Phase 14]
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
    → Per-animation: pose keyframes with easing curves, per-element state snapshots,
      IK chain definitions, physics/constraint parameters, procedural modifier params,
      event markers (name + time for game event triggers),
      animation transition map (from → to pairs for state machine reference)
  → Bevy runtime component reads RON, assembles parts, evaluates animation at 60 FPS
```

Runtime bone export produces smaller textures and smooth full-framerate animation. Requires a Bevy-side runtime component that evaluates the animation pipeline (pose interpolation → IK → constraints → physics → procedural → socket transforms) — this is a separate project with its own documentation. This is the primary export path — high-res line art sprites would produce prohibitively large spritesheets at decent frame rates.

### Secondary: Spritesheet (simple assets, lower priority)

```
Sprite + AnimationSequence
  → Step sequentially from frame 0 at configurable FPS:
    → Full evaluation pipeline (pose interpolation → IK → constraints → procedural → physics → socket walk)
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
- **Build All** button on the project overview: exports every sprite in the project using each sprite's last-used export settings. Shows a progress bar with per-sprite status (e.g., "Exporting soldier (3/12)"). Useful after project-wide changes (palette color edits, stroke taper toggle, Lospec import) that affect all sprites but don't trigger individual sprite file saves
- **Stale export prompt**: When the user modifies a project-wide setting (palette, stroke taper) and then closes a sprite tab or navigates away, a non-blocking toast notification reminds them: "Project settings changed — other sprites may need re-exporting. Use Build All on the project overview." The toast includes a clickable "Build All now" action button. The prompt only appears once per project-wide change (not on every tab close), and is suppressed if the user has already clicked Build All since the change

---

## Undo/Redo

Snapshot-based undo — every mutation captures the full sprite state before and after. Pushed to a single shared history stack (drawing + animation edits combined). Ctrl+Z/Ctrl+Y navigate the stack. The redo stack clears on new actions.

**Drag coalescing**: A continuous drag operation (moving a vertex, adjusting a slider, dragging an element) produces a single undo entry, not one per mouse-move event. The undo snapshot is captured at drag-start; on drag-end, the final state is committed. This means Ctrl+Z after a drag reverts the entire drag in one step, not frame-by-frame. Applies to: element move/rotate/scale drags, vertex drags, control point drags, slider adjustments, transition duration handle drags.

**Viewport changes excluded from undo**: Pan and zoom actions are **not** pushed to the undo stack. The artist should never have to Ctrl+Z through a series of zoom/pan adjustments to get back to a meaningful edit. Viewport state is tracked separately from the undo system.

**Human-readable undo history panel**: A collapsible panel (accessible via menu or hotkey) shows the undo stack as a list of named actions (e.g., "Move element 'left arm'", "Change fill color to index 5", "Insert keyframe at 0.5s", "Delete vertex"). Clicking an entry in the list jumps to that point in history. The current position in the stack is highlighted. This makes undo/redo navigable rather than blind — the artist can see what they're undoing before they undo it.

**Physics & undo**: Undoing a physics/constraint parameter change does not rewind the playhead. Since physics only runs during playback (scrubbing shows pose-interpolated state without physics), there is no stale simulation state — physics will re-simulate correctly from frame 0 the next time playback starts.

---

## Theme Colors

**Dark mode (Twilight 5)**: `#292831` (bg), `#333f58` (panels), `#4a7a96` (accent), `#ee8695` (secondary), `#fbbbad` (text)

**Light mode (Golden Sunset)**: `#ffecd6` (bg), `#ffb873` (panels), `#cb765c` (accent), `#7a4a5a` (secondary), `#25213e` (text)

---

## UI Style

**Icons first, text second.** Default to icons for all buttons, tools, toggles, and actions. Text labels are used only for values, section headers, and places where an icon would be ambiguous. Every icon has a tooltip with its name and keyboard shortcut. The user will provide custom icon assets — do not rely solely on egui built-ins or Unicode symbols.

- **Compact, icon-driven controls** — icons for all tool buttons, layer actions, grid toggles, constraint quick-adds, fill mode selectors, and panel controls. Text only where icons would be unclear (e.g., preset names, numeric values)
- **Sliders with numeric displays** for continuous values (stroke width, zoom, rotation, grid size)
- **Small inline color swatches** — not large color pickers; swatches show palette colors directly
- **Minimal chrome** — panels feel lightweight, not heavy dialog boxes; thin borders, subtle separators
- **Vertically stacked tool options** in sidebar — each option on its own row, not a dense property grid
- **Custom icon assets**: loaded from an icon atlas or individual PNGs. The user provides all icon artwork — the app renders them via egui's image/texture support

### Hybrid Sidebar Layout

The right sidebar has two zones:
- **Top zone (context-sensitive):** Content changes based on the active tool/mode
  - *Line tool* → stroke width slider, curve/straight toggle, active color
  - *Select tool* → position, rotation, scale, origin point, constraints with preset dropdowns and primary/advanced split (IK, physics, look-at, volume preserve, procedural modifiers — shown when element has them), quick-add icon buttons for common effects (Breathing, Sway, etc. — each with a distinct icon and tooltip)
  - *Fill tool* → active color selector
  - *Eyedropper tool* → (minimal — shows sampled color preview)
  - *Eraser tool* → (minimal or empty)
  - *Settings mode* → palette management, theme toggle, grid config, reference image management
- **Bottom zone (fixed tabs):** Always-visible regardless of active tool
  - *Layers tab* → layer list with visibility/lock toggles, add/remove/duplicate/mirror/combine/reorder
  - *Palette tab* → color swatches, add/delete, Lospec importer
  - *Skins tab* → skin list with create/rename/duplicate/delete, per-element override editor

---

## Implementation Phases

Each phase after Foundation adds one testable feature increment. The artist should be able to sit down at the end of each phase and test the new capability.

### Phase 1: Foundation — "I can draw lines on a canvas" ✅

**Icons needed:**
- Line tool, Undo, Redo
- Canvas flip (status bar indicator)
- Zoom to selection
- Grid dots toggle, Grid lines toggle, Grid mode (straight/isometric) toggle
- Taper toggle
- Sprite metrics: element, vertex, layer, animation, atlas size (5 small status bar icons)
- Dark/light theme toggle

**Planned features:**
- Init eframe/egui project, Cargo dependencies, Rust data models with serde
- Save/open/new sprite via `rfd` file dialogs and `serde_json`
- AppShell layout: canvas + top toolbar + hybrid right sidebar (context-sensitive top zone + fixed-tab bottom zone) + status bar
- Canvas rendering with egui `Painter`, viewport pan/zoom, canvas boundary (dashed rectangle)
- Grid system: manual size (1–64 px), toggleable dots and lines (independent), straight and isometric modes. Snapping always active
- Line tool: click to place vertices, auto-curve (Catmull-Rom), double-click to finish. Stroke preview (rubber band). Curve handles (cp1/cp2)
- Stroke taper: project-wide toggle (default on), per-element opt-out
- Auto-merge: fuse elements when vertices coincide on same layer, with visual preview
- Canvas flip (`H`), Zoom to selection (`F`), Hover highlight
- Undo/redo with drag coalescing, viewport changes excluded
- Sprite metrics bar (icon + number pairs in status bar)
- Dark/light theme toggle

**Artist test:** Open app → see gridded canvas → draw tapered strokes → pan/zoom/flip → undo/redo → save and reopen.

**Implementation notes (completed 2026-03-15):**

All planned features implemented except stroke taper rendering (math functions exist, rendering not yet wired up). Key deviations and additions from the original plan:

- **Per-element `curve_mode`**: Added `curve_mode: bool` to `StrokeElement` (with `#[serde(default)]`). Toggled via `C` key during line tool drawing. Curve mode uses Catmull-Rom auto-curves through vertex positions; straight mode uses direct vertex-to-vertex edges with corner fillets.

- **Figma-style corner radius (straight mode)**: Instead of simple straight lines, straight-mode paths use render-time fillet arcs at corners. Tangent distance `d = R / tan(θ/2)` where R = corner radius and θ = angle between edges. Fillet arc approximated by cubic bezier with `ratio = (4/3) * tan(θ/2) * tan(α/4)`. The `min_corner_radius` is a project-level setting (sidebar slider, 0-32).

- **Polyline rendering for straight mode**: Fillet arcs are flattened to polyline points via `flatten_cubic_bezier` and combined with straight-edge segments into a single `PathShape`. This produces proper line joins at arc-edge junctions. Adaptive tolerance `(0.5 / viewport.zoom).max(0.01)` ensures consistent screen-space quality.

- **Isometric diamond lattice snapping**: Grid snap always uses the isometric diamond lattice with basis vectors `u=(2gs, gs)`, `v=(2gs, -gs)` regardless of the `grid_mode` setting. This is intentional — the isometric art workflow requires lattice-aligned snapping for all grid modes.

- **Grid dots on isometric lattice**: Dots render on a staggered diamond grid (even rows at `x=0,±4gs,±8gs...`, odd rows at `x=±2gs,±6gs...`). Grid lines switch between straight (square) and isometric (±0.5 slope diagonals). Both are independently toggleable.

- **Canvas actions architecture**: Canvas returns `CanvasAction` enums (`CommitStroke`, `MergeStroke`); `App::dispatch_action()` handles sprite mutation, undo snapshots, and curve recomputation. No direct sprite mutation from UI code.

- **Auto-close paths**: Double-click near the first vertex closes the path (`closed = true`). Close threshold uses grid snapping distance.

- **Custom icon assets**: Icons loaded via `egui::include_image!` from `assets/icons/`. Icon helper functions in `ui/icons.rs`.

- **File structure**: Canvas code split into three focused files: `canvas.rs` (orchestrator), `canvas_input.rs` (input handling), `canvas_render.rs` (rendering). This avoided the previous implementation's 2,300-line monolithic canvas file.

**Remaining warnings to resolve in future phases:**
- `math.rs: cubic_bezier_eval, approximate_bezier_length, cumulative_arc_lengths` — wire up in stroke taper rendering (Phase 6)
- `math.rs: catmull_rom_to_cubic` — used by curve mode tests; either use in recompute_auto_curves or inline and remove
- `merge.rs: vertex_id` — use in auto-merge target identification or remove field
- `io.rs: save_project, load_project` — use in Phase 15 (project management)
- `theme.rs: origin_color` — use when origin point handle is rendered (Phase 3+)

### Phase 2: Select & Edit — "I can move and arrange what I drew" ✅

**Icons needed:**
- Select tool, Position, Rotation, Scale

**Planned features:**
- Select tool: click, shift-click, drag marquee, Ctrl+A
- Move, scale, rotate with user-defined origin (grid-snapped)
- Copy/paste (Ctrl+C/V, including cross-sprite), Delete
- Hover highlight, selection stack popup (Alt+click with thumbnail previews)
- Straight/curve toggle (`C` key) for existing vertices
- Context-sensitive tool options in sidebar top zone

**Artist test:** Draw elements → select and rearrange → copy/paste → Alt+click to pick buried elements → toggle vertex curves.

**Implementation notes (completed 2026-03-16):**

All planned features implemented. Key additions beyond the original plan:

- **Vertex editing sub-mode**: When exactly one element is selected, individual vertices become visible as dots on the path. Click to select a vertex, drag to reposition (grid-snapped). On curve-mode elements, selecting a vertex shows cp1/cp2 bezier handles that can be dragged to reshape the curve. Keyboard: Delete removes a vertex, R resets a vertex's manual handles back to auto-curve, Escape deselects vertex (then element).

- **Manual handles with minimum radius enforcement**: Added `manual_handles: bool` to `PathVertex`. When a user drags a control point handle, the vertex is marked `manual_handles: true` and `recompute_auto_curves` preserves its user-set CPs instead of overwriting with Catmull-Rom values. Manual handles still enforce a minimum curvature radius using the same angle-based formula as straight-mode fillets: `d = R / tan(θ/2)` where θ is the angle between the two handle directions at the vertex. This prevents visually sharp curves even when handles are dragged close together.

- **Transform handles**: 8 directional scale handles (corners + edges) plus a rotation handle rendered on the selection bounding box. Scale handles resize relative to the opposite corner/edge. Rotation handle (above center) rotates around the AABB center with 15° snap when Shift is held. All handles render as small colored squares/circles with hover feedback.

- **Selection stack popup**: Alt+click on overlapping elements shows a popup listing all elements under the cursor, ordered by render depth. Each entry shows a color swatch and element name. Click to select that specific element. Dismisses on click-outside or Escape.

- **Clipboard**: System clipboard integration via `arboard` crate. Elements serialized as JSON with a `messy_grapefruit_clipboard` sentinel field for cross-sprite paste. Cut/copy/paste all clear vertex selection state. Paste offsets by (+10, +10) and generates new UUIDs for elements and vertices.

- **Context-sensitive sidebar**: Expanded sidebar shows position (X/Y drag values), rotation (degrees), scale (X/Y), stroke width buttons, color index, and curve/straight toggle when elements are selected. Collapsed sidebar shows compact readouts. All property edits go through undo. The sidebar also gained a `theme::with_input_style` helper for consistent dark/light input field styling.

- **Bake transform on curve toggle**: Toggling an element between curve and straight mode in the sidebar (or via `C` key while selected) bakes the current position/rotation/scale into vertex positions and resets the transform to identity before recomputing curves. This prevents visual jumps when switching modes.

- **Canvas code split into 7 sub-functions**: `handle_select_hover`, `handle_select_drag_start`, `handle_select_drag_update`, `handle_select_drag_end`, `handle_select_click`, `handle_select_keyboard`, `render_select_overlays`. Each handles one aspect of the select tool. Vertex editing logic is integrated into these same functions with priority checks (vertex/handle interactions take precedence over transform handle interactions when in vertex edit mode).

**Remaining warnings to resolve in future phases:**
- Same as Phase 1 (unused math functions for taper rendering, unused catmull_rom_to_cubic, unused merge.rs vertex_id, unused io.rs project save/load, unused theme.rs secondary/origin_color)

### Phase 3: Layers — "I can organize my art" ✅

**Status:** Complete.

**Icons added** (`assets/icons/`):
- `layer_remove`, `layer_duplicate`, `layer_mirror`, `layer_combine` — header action buttons
- `layer_solo` — shown on soloed layer row only, click to clear
- `layer_group_collapse`, `layer_group_expand`, `layer_group_create` — group management
- `layer_move_up`, `layer_move_down` — reorder buttons on layer rows and group headers

**What was built:**
- **Layer panel header buttons**: Add, Remove, Duplicate, Mirror (horizontal flip), Combine (merge down), Create Group — all undoable
- **Layer renaming**: right-click layer name → "Rename" (inline TextEdit, Enter to commit, Escape to cancel)
- **Visibility/lock toggles**: per-layer eye and padlock icons, per-group cascade toggles
- **Solo mode**: double-click layer name or canvas element to solo its layer (dims others to ~15% opacity, non-interactive). Solo icon appears only on the soloed layer row. Double-click canvas background or soloed layer name to clear. Solo-aware hit testing, marquee select, and Ctrl+A
- **Layer groups**: collapsible folder headers with visibility/lock cascade. Right-click group header for Rename/Ungroup. Single-level nesting
- **Layer reorder**: up/down arrow buttons on each layer row and group header (moves group as a block). Undoable
- **Group assignment**: right-click layer → "Move to Group >" submenu lists all groups + "None". Also "Remove from Group" shortcut
- **ID-based active layer tracking**: replaced index-based with `active_layer_id: Option<String>`, validated after undo/redo

**Files modified:**
- `src/model/sprite.rs` — `LayerGroup` struct, `group_id` on Layer, `layer_groups` on Sprite, helpers
- `src/state/editor.rs` — Extended `LayerState` (active_layer_id, solo, rename, drag state), resolve/validate helpers
- `src/ui/sidebar_layers.rs` — Full rewrite: header buttons, layer rows, group headers, context menus, up/down reorder
- `src/ui/sidebar.rs` — Updated call chain, collapsed sidebar solo dimming
- `src/ui/icons.rs` — 10 new icon functions
- `src/ui/canvas_render.rs` — Solo dimming (alpha × 0.15 for non-soloed layers)
- `src/ui/canvas_select.rs` — Solo-aware hit testing, double-click canvas for solo toggle
- `src/engine/hit_test.rs` — `solo_layer_id` parameter on hit_test functions
- `src/engine/transform.rs` — `solo_layer_id` parameter on `elements_in_rect`
- `src/ui/canvas.rs`, `src/ui/canvas_input.rs`, `src/main.rs`, `src/clipboard.rs`, `src/ui/toolbar.rs` — Updated to ID-based active layer

**Artist test:** Create multiple layers → draw body parts on separate layers → reorder with up/down buttons → solo a layer (double-click name) → create groups → collapse/expand → move layers between groups via right-click menu.

### Phase 4: Color & Palette — "I can color my line art" ✅

**Status:** Complete.

**Icons added** (`assets/icons/`):
- `tool_fill`, `tool_eyedropper` — tool icons
- `palette_add`, `palette_remove`, `palette_import` — palette panel action buttons
- `settings` — theme settings toggle (same row as dark/light theme icons)

**What was built:**

- **Palette data model**: `PaletteColor` (RGBA), `Palette` (name + colors vec, max 256). Index 0 is always transparent. Default palette is "Downgraded 32" (33 colors). Elements store `fill_color_index: u8` and use palette index for stroke via `color_index: u8`
- **Palette panel**: color swatch grid (16×16 per swatch), click to select stroke color (Line tool) or fill color (Fill tool). Selected swatch has 2px highlight border. Checkerboard background for transparent colors
- **Add/delete colors**: add button inserts white (disabled at 256 max with "Palette Full" tooltip). Delete remaps all element indices across all sprites via `remap_color_index()`
- **RGB color editor**: sliders + DragValue inputs (0–255) for the selected non-transparent color. Dispatches `EditPaletteColor` — all elements using that index update live
- **Recent colors bar**: last 8 used colors shown as 14×14 swatches (session-only, LRU deduplication). Click to select
- **Fill tool** (`G`): click closed element to set `fill_color_index`, click empty canvas to set `background_color_index`. Hover highlight on fillable targets. Tool options panel shows active fill color swatch + mini palette picker
- **Eyedropper tool** (`I`): click element to sample both stroke and fill colors into brush. Click empty canvas to sample background. Alt+click temporary mode from Line or Fill tool — returns to previous tool after sampling. Hover shows 16×16 color swatch tooltip at cursor
- **Lospec palette import**: text input for slug (e.g. "endesga-32"), fetches from `https://lospec.com/palette-list/{slug}.json` via blocking HTTP. Error display with red text. On success: replaces palette, ensures index 0 is transparent, truncates to 256, auto-picks theme colors
- **Fill rendering**: ear-clipping polygon triangulation (`ear_clip_triangulate`) for concave polygon fill. Deduplicates near-identical points from curve flattening. Epsilon tolerance on convexity check for near-collinear curve points. Fill rendered as `egui::Mesh`, stroke rendered as separate `PathShape` on top
- **Editor theme from palette**: `ThemeColorIndices` maps 5 semantic roles (Panel, Canvas, Accent, Highlight, Text) to palette indices. Separate mappings for dark and light mode. `auto_pick_theme_colors()` assigns roles by luminance sorting. Auto-triggered on Lospec import
- **Theme settings UI**: settings icon button next to dark/light toggles in expanded sidebar. Opens role customization panel: 5 role swatches (20×20) in a row, click to open palette picker for that role. "Auto" button for intelligent reassignment
- **Theme application**: `apply_theme()` sets all egui visuals from resolved palette colors — panel backgrounds, canvas background, text/icons, hover/active/selection states, grid dots, transform handles
- **Keyboard shortcut safety**: all single-key shortcuts (tool switching, etc.) check `text_has_focus` to prevent triggering while typing in text fields
- **App defaults persistence**: palette + editor preferences (theme mode, dark/light theme color indices) saved to `<config_dir>/messy-grapefruit/defaults.json` via `dirs` crate. Auto-saved on palette import/add/edit/delete, theme mode toggle, and theme role color changes. Loaded on startup so new projects inherit the last configured palette and theme

**Files added:**
- `src/ui/canvas_fill.rs` — fill tool click/hover logic
- `src/ui/canvas_eyedropper.rs` — eyedropper sampling + temporary mode

**Files modified:**
- `src/model/project.rs` — `PaletteColor`, `Palette`, `ThemeColorIndices`, `EditorPreferences` with dark/light theme indices
- `src/model/sprite.rs` — `fill_color_index` on `StrokeElement`, `background_color_index` on `Sprite`
- `src/state/editor.rs` — `BrushState` (color_index, fill_color_index), recent_colors, lospec state, theme_settings_open, theme_role_picker, eyedropper_return_tool
- `src/action.rs` — `SetFillColor`, `SetBackgroundColor`, `AddPaletteColor`, `DeletePaletteColor`, `EditPaletteColor`, `ImportPalette`
- `src/main.rs` — action dispatch for all palette/fill actions, color remapping on delete, auto theme on import, app defaults load on startup + save on palette/theme changes
- `src/theme.rs` — `ThemeColors` cache, `resolve_from_palette()`, `apply_theme()`, thread-local active theme storage, canvas/grid/handle color getters
- `src/ui/sidebar.rs` — theme toggle buttons + settings icon, theme role swatch row + palette picker, auto-pick button, saves app defaults on theme changes
- `src/ui/sidebar_palette.rs` — full palette grid, add/delete/import buttons, RGB editor, recent colors bar, Lospec import dialog
- `src/ui/sidebar_tools.rs` — fill tool options (swatch + mini picker), eyedropper options (stroke/fill display)
- `src/ui/canvas_render.rs` — `render_filled_path()`, ear-clipping triangulation, fill mesh rendering
- `src/ui/canvas_input.rs` — `G`/`I` key shortcuts, Alt+click eyedropper, `text_has_focus` guard
- `src/ui/icons.rs` — 6 new icon functions (tool_fill, tool_eyedropper, palette_add/remove/import, settings)
- `src/io.rs` — `fetch_lospec_palette()` HTTP fetch + JSON parsing, `AppDefaults` struct, `save_app_defaults()` / `load_app_defaults()` persistence
- `src/engine/hit_test.rs` — `hit_test_fill()` for fill tool targeting
- `src/ui/canvas.rs` — fill/eyedropper tool rendering integration

**Artist test:** Build a palette → fill shapes with color → sample colors with eyedropper → import a Lospec palette → verify all art updates on palette color change → customize theme role colors → switch dark/light mode → verify theme follows palette → close and reopen app → verify palette and theme persisted.

### Phase 5: Drawing Refinement — "I can draw complex art efficiently" ✅

**Status:** Complete.

**Icons added** (`assets/icons/`):
- `tool_eraser`, `tool_snap_vertex` — tool icons
- `symmetry_vertical`, `symmetry_horizontal`, `symmetry_both` — symmetry axis status bar indicators
- `ref_image_import`, `ref_image_lock`, `ref_image_visible` — reference image controls

**What was built:**

- **Snap to vertices**: Magnetic snap to existing vertices on visible/unlocked layers. Applied after grid snap in `get_snap_pos_with_vertex_snap()`. Threshold = `HIT_TEST_THRESHOLD / zoom`. Blue diamond indicator at snap target. Toggle in toolbar. Respects solo mode.
- **Eraser tool** (`E`): Two modes — click vertex (delete + split) or click segment (delete segment, clean up islands). Vertex hit takes priority over segment hit. Red highlight preview on hover. Both resulting elements inherit position/rotation/scale/origin/colors from original. Closed path erase opens the path. Interior vertex erase splits into two elements (first keeps original ID). 6 unit tests for all erase cases.
- **Symmetry drawing** (`S`): Mirror axis (V/H/V+H), dashed guide line on canvas, ghost preview of mirrored stroke at 50% opacity. Cycles via S hotkey: off → V → H → Both → off. Axis defaults to canvas center. Mirrored strokes committed atomically via `CommitSymmetricStrokes` action (single undo step). Vertices reversed for proper winding direction. Status bar shows axis indicator icon. 4 unit tests for mirror math.
- **Reference image overlay**: Import PNG/JPG via toolbar button or drag-and-drop. Renders behind all layers. Per-image controls: position, opacity slider (default 30%), visibility toggle, lock toggle, delete. `ReferenceImage` struct on `Sprite` with `#[serde(default)]` for backward compat. Textures cached in `App.ref_image_textures` HashMap, synced each frame. Selection border shown when selected.

**Files added:**
- `src/engine/eraser.rs` — vertex/segment erase + element splitting logic
- `src/engine/symmetry.rs` — mirror point/vertex/vertices math
- `src/ui/canvas_eraser.rs` — eraser tool UI (hover, click, preview)
- `src/ui/sidebar_reference.rs` — reference image list panel

**Files modified:**
- `src/state/editor.rs` — `Eraser` in `ToolKind`, `EraserHover` enum, `SymmetryAxis`/`SymmetryState`, `RefImageDragState`, new fields on `EditorState` (vertex_snap_enabled, snap_vertex_target, eraser_hover, symmetry, selected_ref_image_id, dragging_ref_image)
- `src/action.rs` — `CommitSymmetricStrokes`, `EraseVertex`, `EraseSegment`, `AddReferenceImage`, `RemoveReferenceImage` actions
- `src/main.rs` — dispatch for all new actions, `ref_image_textures` HashMap on `App`, `sync_ref_image_textures()`, drag-and-drop file handling, `find_element_location()` helper
- `src/engine/mod.rs` — added `eraser`, `symmetry` modules
- `src/engine/snap.rs` — `snap_to_vertex()` function
- `src/engine/hit_test.rs` — `hit_test_segment()`, `hit_test_eraser()` functions
- `src/model/sprite.rs` — `ReferenceImage` struct, `reference_images` field on `Sprite`
- `src/ui/mod.rs` — added `canvas_eraser`, `sidebar_reference` modules
- `src/ui/canvas.rs` — eraser tool dispatch, symmetry axis rendering, ghost preview, vertex snap indicator, ref image rendering, `ref_image_textures` parameter
- `src/ui/canvas_input.rs` — `E`/`S` hotkeys, `handle_line_tool_input` returns `Vec<AppAction>`, `commit_stroke` supports symmetry via `create_mirrored_elements`, `get_snap_pos_with_vertex_snap`
- `src/ui/canvas_render.rs` — `render_vertex_snap_indicator`, `render_symmetry_axis`, `render_symmetry_ghost`, `render_reference_images`
- `src/ui/toolbar.rs` — eraser, vertex snap, symmetry, ref image import buttons
- `src/ui/sidebar.rs` — eraser match arm, reference images panel
- `src/ui/status_bar.rs` — symmetry axis indicator
- `src/ui/icons.rs` — 8 new icon functions
- `src/theme.rs` — `vertex_snap_color`, `eraser_highlight_color`, `symmetry_axis_color`, `symmetry_ghost_color`
- `src/io.rs` — `load_image_texture()` for PNG/JPG via `image` crate

**Artist test:** Import a reference image → draw a symmetrical character over it → use eraser to fix mistakes → verify snap aligns vertices across layers.

### Phase 6: Gradient & Hatch Fills — "I can add visual depth"

**Icons needed:**
- Fill mode toggles: flat (solid square), linear gradient (gradient bar), radial gradient (radial circle), hatch (hatch lines)
- Hatch editor: add layer, remove layer, import patterns, export patterns

- Gradient fill: linear (angle) and radial (center + radius) between two palette colors. Four icon toggles for fill mode
- Hatch fill patterns: project-level library, multi-layer (cross-hatch), live preview swatch as primary editor, visual swatch grid for selection
- Flow curves: bezier guide path to warp hatch lines along element shape, editable on canvas
- Cross-project hatch sharing (`.hatchpatterns` JSON import/export)

**Artist test:** Apply a linear gradient to a copper pot → create a wood-grain hatch pattern → apply with flow curve to a barrel → export pattern to another project.

### Phase 7: Import & Basic Export — "I can get art in and out"

**Icons needed:**
- Import SVG, Export

- Import SVG: parse `<path>` elements, scale modifier, stroke width normalization (1/2/4/8), CIELAB palette matching, preview before confirming
- SVG generation (`svg_gen.rs`): Sprite → SVG string
- PNG rasterization (`rasterize.rs`): SVG → PNG via resvg
- Export preview dialog: atlas preview + settings (FPS, layout, trim, padding), adjust and re-preview

**Artist test:** Import an Inkscape SVG → verify normalized strokes and palette matching → edit imported paths → export as PNG.

### Phase 8: Animation Core — "I can animate my sprites"

**Icons needed:**
- Player controls: play/pause, start over, skip backward, skip forward, loop toggle
- Insert pose, add animation sequence (plus)
- Canvas state: colored dot (blue/green/orange) for status bar

- Timeline with time axis, playhead, pose keyframe markers (with thumbnails ~32×32)
- Animation sequence tabs: create, switch, rename, delete
- Pose-based keyframes: "Insert Pose" captures full sprite state. Easing curve per pose
- Pose interpolation: continuous properties use easing curve, integers use step
- Animation player icons (play/pause, start over, skip backward/forward, loop)
- Canvas state indicator: color-coded border + status bar dot
- Keyframe visual indicators: filled/outlined/dotted diamonds

**Artist test:** Create a 3-pose animation → play it back → verify smooth interpolation → create a second animation sequence → switch between them.

### Phase 9: Animation Workflow — "I can animate efficiently"

**Icons needed:**
- Auto-key toggle
- Onion skinning toggle, onion skin settings (gear)
- Animation templates (dropdown trigger)
- Pose context menu: copy, paste, mirror, duplicate
- Event marker flag

- Auto-key mode: toggle to auto-create/update keyframes on edit
- In-place pose editing: scrub to keyframe, modify directly
- Onion skinning: frame mode + keyframe mode, configurable colors
- Inline easing curve editing: click timeline segment → popup with bezier editor + presets
- Pose copy/paste/mirror: right-click context menu. Mirror for walk cycles
- Transition duration handles: drag gaps between keyframes
- Animation templates: idle, walk cycle, attack, jump
- Animation event markers: named flags on timeline ruler, exported in RON

**Artist test:** Enable auto-key → pose a walk cycle → mirror left-foot to right-foot → adjust easing curves → add "footstep" event markers → use onion skinning to verify flow.

### Phase 10: Layer Sockets — "Child layers follow parent bones"

**Icons needed:**
- Socket attach, socket detach

- Layer sockets: attach a layer to a parent vertex, inherit position + rotation, unlimited chain depth
- Circular reference detection, warning on parent vertex deletion
- Socket visibility: faint dashed connection lines when relevant layers selected

**Artist test:** Draw an arm → draw a weapon on another layer → socket weapon to hand vertex → animate arm → verify weapon follows → chain 3+ layers.

### Phase 11: Skins — "I can create visual variants"

**Icons needed:**
- Skin: create, duplicate, delete
- Override toggle (per-property on/off)

- Skin data model: per-element overrides for strokeColorIndex, fillColorIndex, strokeWidth
- Skin management: create, rename, duplicate, delete. Thumbnail strip selector in toolbar
- Canvas renders with active skin; editing modifies shared base geometry
- Export: separate atlas per skin, shared animation RON with skin manifest

**Artist test:** Create a "red team" skin → override colors → switch between default and skin → export → verify separate atlases, shared animation.

### Phase 12: Inverse Kinematics — "Limbs solve automatically"

**Icons needed:**
- IK chain, IK target (crosshair)
- Bend direction toggle
- Solver type toggle (2-bone / FABRIK)

- 2-bone analytical solver (law of cosines, bend direction)
- FABRIK solver for chains > 2
- IK target: draggable crosshair on canvas, position captured in poses
- FK/IK mix per-pose, angle constraints for 2-bone
- Unit tests for solver math

**Artist test:** Set up arm IK chain → drag hand target → verify elbow solves → flip bend direction → animate IK target across poses → blend FK↔IK.

### Phase 13: Constraints & Dynamics — "My sprites feel alive"

**Icons needed:**
- Quick-add: Breathing (lungs/wave), Sway (wind/curve), Jiggle (vibration), Eye Track (eye), Tail Follow (arc), Bounce (spring)
- Advanced disclosure chevron, Try-it play icon
- Volume preserve toggle, Debug overlay toggle

- Quick-add icon buttons for common effects (one-click presets)
- Constraint UX: behavior presets, artist-friendly labels, primary/advanced split, live preview on slider drag, "Try it" mini-preview
- Spring/jiggle physics (6 presets), squash & stretch (volume preserve)
- Procedural modifiers (Breathing, Floating, Flickering, Wobble, Pulsing)
- Look-at constraint (Snap, Smooth, Lazy presets)
- Full evaluation pipeline: pose → IK → constraints → procedural → physics → socket transforms
- Visual debug overlays, unit tests

**Artist test:** Click "+ Sway" on hair → see it move immediately → adjust Bounciness → click "Try it" → add Breathing to torso → add Eye Track → play full animation.

### Phase 14: Export Pipeline — "I can ship to Bevy"

**Icons needed:**
- Build All, Watcher toggle

- Bone export: per-element PNGs → texture atlas + animation RON (poses, IK, physics, constraints, event markers, transition map, skin manifest)
- Spritesheet export: baked frames → atlas PNG + TextureAtlasLayout RON
- Auto-export on save (debounced), file watcher with `notify` (re-exports changed sprite only)
- Build All: batch export every sprite, progress bar, stale export toast after project-wide changes

**Artist test:** Export a rigged character → load in Bevy → verify hot-reload → change palette → see stale export toast → Build All → verify all sprites re-exported.

### Phase 15: Project & Polish — "Production-ready workflow"

**Icons needed:**
- New sprite (visual size presets — 4 squares at relative sizes)
- Game-resolution preview toggle
- Undo history panel toggle
- Hint dismiss (X), Searchable shortcut overlay trigger
- Crash recovery restore/discard
- State machine: auto-layout

- Project overview: live compose preview with draggable sprites, per-sprite animation/skin selection
- New sprite dialog: clickable visual size presets + freeform input
- Project file save/load, file dialogs (`rfd`)
- Game-resolution preview window (`P` hotkey): 1:1 export resolution, real-time sync
- Animation state machine preview: node graph, transition arrows, exported in RON
- Keyboard shortcuts in tooltips + searchable overlay (`?`)
- Crash recovery (`.sprite.recovery` files, restore prompt on launch)
- Undo history panel (named actions, click to jump)
- First-time contextual hints (non-blocking, dismissable)

**Artist test:** Manage a 5-sprite project → preview sprites composed together → preview at game resolution → crash-recover → use shortcut overlay.

---

## Testing Strategy

- **Unit tests on engine math**: IK solvers (law of cosines, FABRIK convergence, angle constraints, bend direction), spring integrator (convergence, energy conservation), angle wrapping (±π), Catmull-Rom → cubic bezier conversion, procedural waveform generators. These are pure functions — easy to test, high regression value.
- **Visual debug overlays**: Toggleable canvas overlays that render bone chains, IK targets, constraint gizmos, and spring targets. Not automated, but essential for authoring and debugging procedural animation. Built during Phase 7.
- **Round-trip save/load tests**: If serialization bugs appear, add targeted tests for `.sprite` / `.spriteproj` round-trips via serde.

---

## Verification

1. **Drawing**: Open app → see dot grid → draw lines with auto-curve → verify stroke preview (rubber band) shows next segment with curve shaping before click → verify snap to grid → drag curve handles → approach an existing vertex → verify merge preview highlights target → confirm auto-merge fuses elements when vertices coincide → verify vertex IDs are stable after merge
1a. **Canvas flip**: Press `H` → verify viewport flips horizontally → verify status bar shows flip indicator → draw a vertex → verify it snaps to correct grid position (not mirrored coordinates) → press `H` again → verify viewport returns to normal → verify no data was modified
1b. **Zoom to selection**: Select an element → press `F` → verify viewport zooms/pans to frame the element → select multiple elements → press `F` → verify viewport frames all selected → deselect all → press `F` → verify viewport frames all visible content
1c. **Snap to vertices**: Draw an element on layer 1 → create layer 2 → draw near an existing vertex on layer 1 → verify magnetic snap indicator appears → verify vertex snaps to target position → toggle snap off → verify vertex follows grid only → verify snap indicator distinguishes from merge preview (different color)
2. **Palette**: Import lospec palette by slug → draw with indexed colors → change a palette color → verify all art using that index updates → verify 256 color max is enforced
2a. **Recent colors**: Draw with color index 5 → draw with color index 12 → draw with color index 3 → verify recent colors bar shows indices 3, 12, 5 (most recent first) → click color 12 in recent bar → verify it becomes active color → verify recent bar updates
3. **Layers**: Add layers → draw on different layers → toggle visibility → reorder → combine → duplicate → mirror horizontally → verify rendering order
3a. **Solo mode**: Create 3 layers with elements → click solo on layer 2 → verify layers 1 and 3 dim to ~15% opacity → verify layer 2 stays full opacity → shift+click solo on layer 3 → verify both layers 2 and 3 are soloed → click "clear solo" → verify all layers return to normal opacity → verify solo mode doesn't affect export
4. **Selection**: Click to select → shift-click multi-select → Ctrl+A select all → drag marquee → Ctrl+C/V copy/paste (including cross-sprite paste) → Delete to remove → verify origin point is draggable and grid-snapped
5. **Fill**: Fill closed path → verify fillColorIndex set → fill empty canvas → verify backgroundColorIndex set → verify background renders in export
5a. **Eyedropper**: Draw with color index 7 → switch to eyedropper (`I`) → click the element → verify active color changes to index 7 → switch to line tool → hold Alt → click a filled element → verify stroke color sampled → Shift+click → verify fill color sampled instead
6. **Eraser**: Delete mid-path vertex → verify path splits into two elements → verify existing pose keyframes are updated with split element data → click a line segment between two vertices → verify segment removed but vertices remain (if still connected to other segments) → delete a segment whose endpoint connects to nothing else → verify the island vertex is automatically removed
6a. **Reference image**: Import a PNG reference → verify it renders behind all layers → adjust opacity slider → verify transparency changes → drag to reposition → lock the reference → verify it can't be selected/moved → toggle visibility off → verify it disappears → export sprite → verify reference image is not included in export
7. **Animation**: Insert pose keyframes at different times → set different easing presets per pose → play animation → use skip forward/backward → verify interpolation between poses → verify color indices snap (step interpolation) → verify rotation/scale pivots around origin → insert pose keyframe past duration → verify duration auto-extends
7a. **Auto-key mode**: Enable auto-key → move playhead to empty time → modify an element → verify keyframe created automatically → move playhead to existing keyframe → modify element → verify keyframe updated in-place (not duplicated) → disable auto-key → modify element → verify no keyframe created → verify red tint indicator appears/disappears with toggle
7b. **In-place editing**: Scrub playhead to existing keyframe → verify canvas shows that pose's state → modify element position → verify keyframe updates without needing re-insert → scrub away and back → verify modification persisted
7c. **Interpolated state**: Create two poses at different times with different positions → scrub to midpoint → verify canvas shows interpolated pose → insert new keyframe at midpoint → verify it captures the interpolated state as its starting point → modify from there → verify only the delta is needed
7d. **Pose copy/paste/mirror**: Create a pose → right-click keyframe → copy → move playhead → paste → verify identical pose at new time → mirror a pose with asymmetric positions → verify positions flip horizontally and rotations negate → use mirrored poses to build a walk cycle → verify left/right symmetry
7e. **Timeline UX**: Verify pose thumbnails appear on keyframe markers and update when poses change → drag gap between two keyframes → verify transition timing adjusts → Shift+drag → verify only right keyframe moves → select animation template (e.g., walk cycle) → verify keyframes created with correct timing and easing → verify all poses start from current sprite state
7f. **Onion skinning keyframe mode**: Enable keyframe mode onion skinning → place keyframes far apart → verify ghosts show previous/next keyframe poses (not time-adjacent frames) → enable both frame mode and keyframe mode → verify both ghost sets appear simultaneously
8. **Layer sockets**: Draw an arm element → draw a weapon on a separate layer → socket the weapon layer to a vertex on the arm → animate the arm → verify weapon follows → chain a third layer to the weapon → verify full chain works → try creating a circular reference → verify it's rejected → delete the socket vertex → verify warning and child detaches to world-space position
9. **IK**: Create a 2-bone socket chain (upper arm → forearm → hand) → set up IK chain → drag IK target → verify joints solve correctly → flip bend direction → verify elbow flips → insert pose keyframes with IK target at different positions → play → verify smooth tracking → set IK mix to different values across poses → verify FK-to-IK transition → set angle constraints → verify elbow respects limits → create a 4-bone chain → switch to FABRIK → verify it solves
10. **Spring physics**: Click "+ Sway" quick-add on a socketed layer → verify "Hair/Cape sway" preset applied with sensible defaults → verify live preview runs on canvas while adjusting Bounciness slider → click "Try it" button → verify 2-second isolated loop plays → switch preset to "Heavy bounce" → verify parameters update → toggle Advanced → adjust Weight → verify gravity effect → play full animation → verify child overshoots and settles → restart → verify spring state resets
11. **Squash & stretch**: Enable volume-preserve on an element → insert pose with scale.y squash → verify scale.x automatically compensates → verify it works during animation playback
12. **Procedural modifiers**: Click "+ Breathing" quick-add → verify Breathing preset applied (sine on scale.y, low speed, small intensity) → verify live preview shows breathing motion without pressing play → adjust Intensity slider → verify canvas updates in real-time → switch preset to "Floating" → verify property changes to position.y → toggle Advanced → verify Phase and Blend mode revealed → click "Try it" → verify isolated 2-second loop → add a second modifier (Wobble) → verify both layer additively
13. **Look-at**: Click "+ Eye Track" quick-add → verify Smooth tracking preset applied → verify target picker shown → set target element → verify rotation follows target with live preview → drag Smoothness slider toward 0 → verify tracking becomes snappier in real-time → drag toward 1 → verify lazy follow → toggle Advanced → set angle limits → verify clamping → move target past limits → verify element stops at limit
14. **Lospec import**: Import a palette → verify it replaces the current one → verify existing elements remap by index → import a shorter palette → verify out-of-range indices fall back to transparent
15. **Skins**: Create a skin → override strokeColorIndex and fillColorIndex on several elements → switch between default and skin in the dropdown → verify canvas updates to show skin overrides → verify drawing modifies base geometry (shared) while rendering with skin → duplicate a skin → modify the duplicate → verify original is unchanged → delete a skin → verify undo restores it → export → verify separate atlas per skin and shared animation RON with skin manifest
16. **Export (runtime bone)**: Save sprite → check output directory for texture atlas + RON animation data → verify per-element part PNGs are packed correctly → verify RON contains pose keyframes with element states, IK chains, physics params, skin manifest → test in a Bevy project with runtime evaluator + hot-reload → verify socketed layers and procedural animation work at 60 FPS → verify skin switching loads correct atlas
17. **Export (spritesheet, if implemented)**: Export a simple VFX sprite → verify atlas PNG + TextureAtlasLayout RON → verify configurable FPS → verify physics bakes correctly via sequential evaluation
18. **Autosave**: Make changes → wait 3 seconds → verify file saved automatically → switch tabs → verify save triggers → verify no "unsaved changes" dialogs
19. **Navigation**: Double-click sprite card → verify editor tab opens → open multiple sprites → verify tabs work → verify project overview stays as first tab → on project overview, verify sprites render animations live → switch animation sequence and skin via dropdowns → verify preview updates → drag sprites to compose them together
20. **Watcher**: Start watcher → modify and save a .sprite file externally → verify only that sprite re-exports (not all sprites)
21. **Undo + physics**: Change a spring parameter mid-animation → undo → verify playhead stays at current position (FK-only pose) → replay → verify simulation re-runs correctly from frame 0
22. **Canvas state indicator**: Select an animation → scrub playhead to a keyframe → verify green border appears → scrub between keyframes → verify orange border → deselect animation → verify blue border (rest pose) → verify status bar shows matching text label
23. **Hover highlight**: Activate select tool → move cursor over an element → verify highlight outline appears before clicking → move to overlapping area → verify topmost unlocked element highlights → lock that element → verify next element down highlights instead
24. **Selection stack popup**: Draw 3 overlapping elements → Alt+click in the overlap area → verify popup appears listing all 3 elements → click a buried element in the list → verify it becomes selected → verify regular click (no Alt) selects the topmost element as usual
25. **Undo drag coalescing**: Drag an element across the canvas → release → Ctrl+Z → verify element returns to pre-drag position in one step (not incremental) → verify slider adjustments also coalesce into single undo entries
26. **Viewport undo exclusion**: Pan and zoom around the canvas → Ctrl+Z → verify the undo reverts the last data edit (not the pan/zoom) → verify viewport position is unchanged after undo
27. **Keyframe visual indicators**: Create keyframes → scrub playhead onto a keyframe → verify filled diamond marker → scrub between keyframes → verify dotted diamond appears at playhead → verify outlined diamonds for non-active keyframes
28. **Socket visibility**: Create a socket chain (parent → child) → select the child layer → verify faint dashed line drawn from parent vertex to child origin → deselect → verify connection lines disappear → select parent layer → verify lines also appear
29. **Configurable onion skin colors**: Enable onion skinning → open onion skin settings → change "previous" color from red to blue → verify ghost frames update to blue → change "next" color → verify update → verify defaults are red/green
30. **Inline easing curve editing**: Click the timeline segment between two keyframes → verify popup appears with bezier curve editor → drag control points → verify canvas preview updates live → click an easing preset button → verify curve updates → close popup → play animation → verify easing matches what was set
31. **Keyboard shortcuts**: Hover over any tool button → verify tooltip shows keyboard shortcut → press `?` → verify searchable overlay appears → type "zoom" → verify zoom shortcuts are filtered → press Escape → verify overlay closes
32. **Crash recovery**: Edit a sprite → wait for autosave → verify `.sprite.recovery` file exists alongside `.sprite` file → simulate crash (kill process) → relaunch → verify recovery prompt appears → accept restore → verify sprite matches pre-crash state → close cleanly → verify `.sprite.recovery` is deleted
33. **Undo history panel**: Make several edits → open undo history panel → verify list of named actions (e.g., "Move element", "Insert keyframe") → click an earlier entry → verify state jumps to that point → verify current position is highlighted → make a new edit → verify redo entries are cleared from the panel
34. **First-time hints**: Launch app for the first time (or reset hints) → verify a hint bubble appears near a relevant UI element → dismiss the hint → verify it doesn't reappear → verify only 1 hint shows at a time → enable "Don't show again" → verify no more hints appear
35. **Symmetry drawing**: Enable vertical symmetry (`S`) → verify mirror axis appears centered on canvas → draw a stroke on the left side → verify mirrored stroke appears on the right → verify mirrored vertices have independent IDs → drag mirror axis to a new position → verify subsequent draws mirror around the new axis → toggle symmetry off → verify drawing is single-sided again → enable V+H → verify four-way mirroring
36. **Color ramp finder**: Import a palette with multiple shades of blue → select one blue → verify ramp finder shows the other blues sorted light-to-dark → click a shade in the ramp → verify it becomes the active color → select a color with no related shades → verify ramp shows only the base color
37. **Layer groups**: Create a group → drag 3 layers into it → verify layers appear nested under the group → collapse the group → verify child layers are hidden and count badge shows "(3)" → toggle group visibility off → verify all 3 child layers become invisible → toggle one child layer visible individually → verify it overrides the group → drag a layer out of the group → verify it becomes ungrouped → delete the group → verify child layers are preserved as ungrouped → create a group and Shift+Delete → verify group and all children are removed → undo → verify group and children restored
38. **Stroke taper**: Draw an open path → verify stroke tapers to zero at both endpoints with full width at center → draw a closed path → verify uniform stroke width (no taper) → toggle project-wide taper off → verify all open paths switch to uniform width → toggle back on → verify taper returns → select a specific element → set taperOverride to off → verify that element renders uniform while others taper → remove the override → verify element follows project default again → verify taper is visible in export
39. **Game-resolution preview**: Open a 256x256 sprite → zoom in to 400% → press `P` → verify floating preview window appears showing the sprite at 1:1 (256x256 pixels) → draw a stroke on the main canvas → verify preview updates in real-time → play an animation → verify preview shows animated frames → switch skin → verify preview reflects skin → close preview → press `P` again → verify it reopens → verify preview is not included in saved project file
40. **Animation event markers**: Create an animation → right-click timeline ruler → add an event marker named "footstep" → verify labeled flag appears above the time axis → drag the marker to a new time → verify it moves → add a second marker "spawn_projectile" → verify both display → right-click a marker → rename it → verify label updates → delete a marker → verify it's removed → export → verify RON data includes event markers with names and times → undo marker deletion → verify marker restored
41. **Animation state machine preview**: Create 3 animations (idle, walk, attack) → open state machine view → verify 3 nodes appear with animation names → drag from idle node edge to walk node → verify arrow appears → drag from walk to idle → verify bidirectional arrows → drag from idle to attack, attack to idle → verify all transitions shown → right-click an arrow → delete it → verify arrow removed → right-click empty space → auto-layout → verify nodes rearrange → add a new animation → verify a new node appears automatically → delete an animation → verify its node and connected arrows are removed → export → verify RON includes transition map with from/to animation ID pairs
42. **Import SVG paths**: Create a project with a palette containing red, blue, black → File → Import SVG → select an SVG with multiple `<path>` elements, a `<rect>` (unsupported), and a `<text>` (unsupported) → verify import dialog appears with preview → adjust scale modifier to 0.5x → verify preview paths shrink → verify stroke widths in preview show normalized values (e.g., 3px SVG stroke → 4px after normalization) → verify fill/stroke colors in preview match nearest palette entries → confirm import → verify new layer created with StrokeElement per path → verify `<rect>` and `<text>` were skipped with toast notification → verify imported strokes have taper applied (project default on) → select an imported element → verify it's fully editable (drag vertices, adjust handles) → verify stroke width is a standard step (1/2/4/8) → verify color indices reference existing palette colors
43. **Build All & stale export prompt**: Create a project with 3 sprites, each previously exported → change a palette color → close the sprite tab → verify toast notification appears: "Project settings changed — other sprites may need re-exporting" with "Build All now" button → dismiss the toast → open project overview → click "Build All" → verify progress bar shows per-sprite status → verify all 3 sprites re-export with updated palette → verify output files reflect the color change → change palette again → click "Build All now" on the toast → verify batch export runs immediately → close and reopen a sprite tab without changing project settings → verify no stale export toast appears
44. **Sprite metrics bar**: Open a sprite → verify status bar shows element count, vertex count, layer count, animation count, and estimated atlas size → draw a new element with 5 vertices → verify element count increments by 1 and vertex count increments by 5 → add a layer → verify layer count increments → add an animation → verify animation count increments → delete an element → verify counts update immediately → change export settings (e.g., padding) → verify estimated atlas size recalculates → open a different sprite → verify metrics reflect the new sprite's data
45. **Gradient fill**: Draw a closed element → select it → change fill mode to "Linear gradient" → pick start color (palette index 3) and end color (palette index 7) → verify gradient renders on canvas → adjust angle slider → verify gradient direction changes → switch to "Radial gradient" → drag center handle on canvas → verify gradient center moves → verify gradient appears in game-resolution preview → verify gradient renders in export → create a skin → override gradient colors → verify skin shows different gradient → animate gradient colors across two keyframes → verify color indices step-interpolate between poses
46. **Hatch fill patterns**: Open project settings → Hatch Patterns tab → create a new pattern "Wood Grain" → set angle 10°, spacing 4px, width 1px, color index 2 → verify live preview swatch → add a second hatch layer at 80° for cross-hatch → verify preview updates → apply pattern to a closed element → verify hatch lines render within element boundary → add a flow curve → drag control points → verify hatch lines bend to follow the curve → create a curved element (e.g., barrel shape) → apply hatch with flow curve → verify lines follow the curvature naturally → export pattern as `.hatchpatterns` file → create a new project → import the file → verify "Wood Grain" pattern appears in the new project's library → verify hatch fills render correctly in export
