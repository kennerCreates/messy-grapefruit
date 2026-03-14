import type { Component } from "solid-js";

const SelectToolOptions: Component = () => {
  return (
    <div class="tool-options">
      <div class="panel-section-header">Select Tool</div>
      <div class="placeholder">
        Click to select, Shift+click for multi-select
      </div>
    </div>
  );
};

export default SelectToolOptions;
