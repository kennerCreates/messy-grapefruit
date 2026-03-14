import { type Component, createMemo, For, Show } from "solid-js";
import { editorStore, setActiveColor } from "@/stores/editor";
import { projectStore } from "@/stores/project";
import { paletteColors, colorToCSS, addColor } from "@/stores/palette";
import { IconAdd } from "../../assets/icons";
import { MAX_PALETTE_COLORS } from "@/lib/constants";
import type { PaletteColor } from "@/lib/types";

const PalettePanel: Component = () => {
  const colors = createMemo(() => paletteColors());
  const activeColorIndex = createMemo(() => editorStore.activeColorIndex);

  const handleSwatchClick = (index: number) => {
    setActiveColor(index);
  };

  const handleAddColor = () => {
    if (colors().length >= MAX_PALETTE_COLORS) return;
    // Add a default mid-gray color
    addColor({ r: 180, g: 180, b: 180, a: 255 });
  };

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div
        class="palette-grid"
        style={{ flex: "1", "min-height": "0", "overflow-y": "auto" }}
      >
        <For each={colors()}>
          {(color, index) => (
            <button
              class={`palette-swatch ${index() === activeColorIndex() ? "active" : ""} ${index() === 0 ? "transparent" : ""}`}
              style={index() > 0 ? { background: colorToCSS(color) } : undefined}
              onClick={() => handleSwatchClick(index())}
              title={
                index() === 0
                  ? "Transparent (index 0)"
                  : `Color ${index()} — rgb(${color.r}, ${color.g}, ${color.b})`
              }
            />
          )}
        </For>
      </div>

      <div class="palette-footer">
        <span>
          {colors().length}/{MAX_PALETTE_COLORS}
        </span>
        <button
          class="icon-btn"
          onClick={handleAddColor}
          disabled={colors().length >= MAX_PALETTE_COLORS}
          title="Add Color"
        >
          <IconAdd size={16} />
        </button>
      </div>
    </div>
  );
};

export default PalettePanel;
