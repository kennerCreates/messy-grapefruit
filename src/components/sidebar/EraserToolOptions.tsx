import type { Component } from "solid-js";

const EraserToolOptions: Component = () => {
  return (
    <div class="tool-options">
      <div class="panel-section-header">Eraser Tool</div>
      <div class="placeholder">
        Click a vertex to delete it and connected segments.
      </div>
    </div>
  );
};

export default EraserToolOptions;
