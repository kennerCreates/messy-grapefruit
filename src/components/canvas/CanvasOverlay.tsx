import type { Component } from "solid-js";

/**
 * Transparent overlay for CSS-based UI elements on top of the canvas.
 * Minimal for Phase 1 — will be extended for selection handles,
 * curve handle dragging, merge preview indicators, etc.
 */
const CanvasOverlay: Component = () => {
  return (
    <div
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: "100%",
        "pointer-events": "none",
      }}
    />
  );
};

export default CanvasOverlay;
