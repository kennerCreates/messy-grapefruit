import { type Component, createMemo } from "solid-js";
import { editorStore, setStrokeWidth, toggleCurveMode } from "@/stores/editor";
import { activeColor, colorToCSS } from "@/stores/palette";

const LineToolOptions: Component = () => {
  const strokeWidth = createMemo(() => editorStore.strokeWidth);
  const curveMode = createMemo(() => editorStore.curveMode);
  const activeColorIndex = createMemo(() => editorStore.activeColorIndex);

  const currentColor = createMemo(() => {
    const c = activeColor();
    return c ? colorToCSS(c) : "transparent";
  });

  const handleStrokeWidthChange = (e: Event) => {
    const val = parseFloat((e.target as HTMLInputElement).value);
    setStrokeWidth(val);
  };

  return (
    <div class="tool-options">
      <div class="panel-section-header">Line Tool</div>

      {/* Stroke width */}
      <div class="tool-option-row">
        <span class="tool-option-label">Width</span>
        <input
          type="range"
          min="0.5"
          max="20"
          step="0.5"
          value={strokeWidth()}
          onInput={handleStrokeWidthChange}
        />
        <span class="tool-option-value">{strokeWidth().toFixed(1)}</span>
      </div>

      {/* Curve / Straight toggle */}
      <div class="tool-option-row">
        <span class="tool-option-label">Mode</span>
        <button
          class={`mode-toggle ${curveMode() ? "active" : ""}`}
          onClick={toggleCurveMode}
        >
          {curveMode() ? "Curve" : "Straight"}
        </button>
        <span style={{ "font-size": "10px", opacity: "0.5" }}>(C)</span>
      </div>

      {/* Active color */}
      <div class="tool-option-row">
        <span class="tool-option-label">Color</span>
        <div
          class="color-swatch-inline"
          style={{
            background:
              activeColorIndex() === 0
                ? "repeating-conic-gradient(#ccc 0% 25%, transparent 0% 50%) 50% / 8px 8px"
                : currentColor(),
          }}
          title={`Color index ${activeColorIndex()}`}
        />
        <span class="tool-option-value">#{activeColorIndex()}</span>
      </div>
    </div>
  );
};

export default LineToolOptions;
