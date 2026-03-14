import type { Component } from "solid-js";

const FillToolOptions: Component = () => {
  return (
    <div class="tool-options">
      <div class="panel-section-header">Fill Tool</div>
      <div class="placeholder">
        Click closed paths to fill, or click empty canvas to set background
      </div>
    </div>
  );
};

export default FillToolOptions;
