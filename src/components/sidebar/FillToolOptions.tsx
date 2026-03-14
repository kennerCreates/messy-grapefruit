import { type Component, createMemo } from "solid-js";
import { editorStore } from "@/stores/editor";
import { activeColor, colorToCSS } from "@/stores/palette";

const FillToolOptions: Component = () => {
  const activeColorIndex = createMemo(() => editorStore.activeColorIndex);

  const currentColorCSS = createMemo(() => {
    const c = activeColor();
    return c ? colorToCSS(c) : "transparent";
  });

  return (
    <div class="tool-options">
      <div class="panel-section-header">Fill Tool</div>

      {/* Large color preview */}
      <div class="tool-option-row" style={{ "justify-content": "center", "margin-bottom": "12px" }}>
        <div
          class="color-swatch-inline"
          style={{
            width: "48px",
            height: "48px",
            "border-radius": "6px",
            background: activeColorIndex() === 0
              ? "repeating-conic-gradient(#ccc 0% 25%, transparent 0% 50%) 50% / 8px 8px"
              : currentColorCSS(),
          }}
          title={`Active color index ${activeColorIndex()}`}
        />
      </div>

      <div style={{ "text-align": "center", "font-size": "11px", "margin-bottom": "8px", opacity: "0.7" }}>
        Color index: #{activeColorIndex()}
      </div>

      <div class="placeholder" style={{ "font-size": "11px", "text-align": "center" }}>
        Click closed path to fill. Click empty area to set background.
      </div>
    </div>
  );
};

export default FillToolOptions;
