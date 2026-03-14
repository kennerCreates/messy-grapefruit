# CLAUDE.md

## Project
SVG sprite drawing & animation tool. Native Rust desktop app using eframe/egui. See `plan.md` for full spec.

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
