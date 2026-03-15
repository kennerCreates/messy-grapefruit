# CLAUDE.md

## Project
SVG sprite drawing & animation tool. Native Rust desktop app using eframe/egui. See `plan.md` for full spec.

## Design Philosophy
This is an **artist-forward tool**. Every decision — UI layout, feature design, default behavior — must prioritize the artist's creative workflow. The tool should feel like a drawing app, not a developer tool. When in doubt, choose the option that feels most natural to an artist.

- **Icons over text**: default to icons for all buttons, tools, toggles, and actions. Text only for values, section headers, and where icons would be ambiguous. The user provides custom icon assets.
- **Direct manipulation over forms**: drag on canvas, not type in fields.
- **Presets over raw parameters**: offer named behaviors first, expose raw values under "Advanced".
- **Minimal friction**: reduce clicks, reduce dialogs, keep the artist in flow.

## Tech Stack
- **Rust** (edition 2024) with eframe/egui
- JSON file format (`.sprite`, `.spriteproj`) via serde

## Build & Run
```
cargo run
cargo test
cargo clippy
```

## Architecture
- `src/model/` — data types (Sprite, Layer, Element, PathVertex, Project, Palette)
- `src/state/` — editor state, project state, undo/redo history
- `src/ui/` — egui UI modules (canvas, toolbar, sidebar, timeline, status bar)
- `src/engine/` — core logic (grid snapping, hit testing, merge, IK, physics, constraints)
- `src/export/` — SVG generation, rasterization, bone export, spritesheet packing
- `src/io.rs` — file I/O, Lospec palette fetch
- `src/math.rs` — bezier math, Catmull-Rom conversion
- `src/theme.rs` — dark/light theme colors

## Conventions
- All IDs are UUID v4 strings
- Palette index 0 is always transparent
- Vertex animation uses stable vertex IDs, not positional indices
- Undo is snapshot-based (full sprite state before/after)
- Canvas coordinates use Vec2 { x: f32, y: f32 }
