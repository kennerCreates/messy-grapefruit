import { type Component, createMemo, Show } from "solid-js";
import { editorStore } from "@/stores/editor";
import { getActiveSprite, updateSprite } from "@/stores/project";
import { activeColor, colorToCSS, paletteColors } from "@/stores/palette";
import { history } from "@/lib/history";
import type { StrokeElement } from "@/lib/types";

const SelectToolOptions: Component = () => {
  const selectedCount = createMemo(() => editorStore.selectedElementIds.length);

  const selectedStroke = createMemo((): StrokeElement | null => {
    if (editorStore.selectedElementIds.length !== 1) return null;
    const id = editorStore.selectedElementIds[0];
    const sprite = getActiveSprite();
    if (!sprite) return null;
    for (const layer of sprite.layers) {
      for (const el of layer.elements) {
        if (el.id === id && el.type === "stroke") return el as StrokeElement;
      }
    }
    return null;
  });

  const findElementLocation = (elementId: string): { spriteId: string; layerIndex: number; elementIndex: number } | null => {
    const sprite = getActiveSprite();
    if (!sprite) return null;
    for (let li = 0; li < sprite.layers.length; li++) {
      for (let ei = 0; ei < sprite.layers[li].elements.length; ei++) {
        if (sprite.layers[li].elements[ei].id === elementId) {
          return { spriteId: sprite.id, layerIndex: li, elementIndex: ei };
        }
      }
    }
    return null;
  };

  const updateProperty = <K extends keyof StrokeElement>(prop: K, value: StrokeElement[K]) => {
    const el = selectedStroke();
    if (!el) return;
    const loc = findElementLocation(el.id);
    if (!loc) return;

    const oldValue = JSON.parse(JSON.stringify(el[prop]));
    const newValue = JSON.parse(JSON.stringify(value));

    history.execute({
      description: `Change ${prop} on "${el.name || el.id}"`,
      execute: () => {
        updateSprite(loc.spriteId, (s) => {
          const target = s.layers[loc.layerIndex].elements[loc.elementIndex];
          if (target.type === "stroke") {
            (target as any)[prop] = JSON.parse(JSON.stringify(newValue));
          }
        });
      },
      undo: () => {
        updateSprite(loc.spriteId, (s) => {
          const target = s.layers[loc.layerIndex].elements[loc.elementIndex];
          if (target.type === "stroke") {
            (target as any)[prop] = JSON.parse(JSON.stringify(oldValue));
          }
        });
      },
    });
  };

  const handleNumberInput = (prop: keyof StrokeElement, e: Event) => {
    const val = parseFloat((e.target as HTMLInputElement).value);
    if (!isNaN(val)) {
      updateProperty(prop, val as any);
    }
  };

  const handleVec2Input = (prop: "position" | "scale" | "origin", axis: "x" | "y", e: Event) => {
    const el = selectedStroke();
    if (!el) return;
    const val = parseFloat((e.target as HTMLInputElement).value);
    if (isNaN(val)) return;
    const current = { ...el[prop] };
    current[axis] = val;
    updateProperty(prop, current);
  };

  const handleColorSwatchClick = (prop: "strokeColorIndex" | "fillColorIndex") => {
    const activeIdx = editorStore.activeColorIndex;
    updateProperty(prop, activeIdx as any);
  };

  const colors = createMemo(() => paletteColors());

  const strokeColorCSS = createMemo(() => {
    const el = selectedStroke();
    if (!el) return "transparent";
    const c = colors()[el.strokeColorIndex];
    return c ? colorToCSS(c) : "transparent";
  });

  const fillColorCSS = createMemo(() => {
    const el = selectedStroke();
    if (!el) return "transparent";
    const c = colors()[el.fillColorIndex];
    return c ? colorToCSS(c) : "transparent";
  });

  return (
    <div class="tool-options">
      <div class="panel-section-header">Select Tool</div>

      <Show when={selectedCount() === 0}>
        <div class="placeholder">
          Click to select, Shift+click for multi-select
        </div>
      </Show>

      <Show when={selectedCount() > 0}>
        <div style={{ "font-size": "11px", "margin-bottom": "8px", opacity: "0.7" }}>
          {selectedCount()} element{selectedCount() !== 1 ? "s" : ""} selected
        </div>
      </Show>

      <Show when={selectedStroke()}>
        <div class="element-props">
          {/* Position */}
          <div class="tool-option-row">
            <span class="tool-option-label">Pos X</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.position.x.toFixed(1)}
              onChange={(e) => handleVec2Input("position", "x", e)}
              step="1"
            />
          </div>
          <div class="tool-option-row">
            <span class="tool-option-label">Pos Y</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.position.y.toFixed(1)}
              onChange={(e) => handleVec2Input("position", "y", e)}
              step="1"
            />
          </div>

          {/* Rotation */}
          <div class="tool-option-row">
            <span class="tool-option-label">Rotation</span>
            <input
              type="number"
              class="prop-input"
              value={(selectedStroke()!.rotation * (180 / Math.PI)).toFixed(1)}
              onChange={(e) => {
                const deg = parseFloat((e.target as HTMLInputElement).value);
                if (!isNaN(deg)) updateProperty("rotation", deg * (Math.PI / 180));
              }}
              step="1"
            />
            <span class="tool-option-value" style={{ "min-width": "12px" }}>&deg;</span>
          </div>

          {/* Scale */}
          <div class="tool-option-row">
            <span class="tool-option-label">Scale X</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.scale.x.toFixed(2)}
              onChange={(e) => handleVec2Input("scale", "x", e)}
              step="0.1"
            />
          </div>
          <div class="tool-option-row">
            <span class="tool-option-label">Scale Y</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.scale.y.toFixed(2)}
              onChange={(e) => handleVec2Input("scale", "y", e)}
              step="0.1"
            />
          </div>

          {/* Origin */}
          <div class="tool-option-row">
            <span class="tool-option-label">Origin X</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.origin.x.toFixed(1)}
              onChange={(e) => handleVec2Input("origin", "x", e)}
              step="1"
            />
          </div>
          <div class="tool-option-row">
            <span class="tool-option-label">Origin Y</span>
            <input
              type="number"
              class="prop-input"
              value={selectedStroke()!.origin.y.toFixed(1)}
              onChange={(e) => handleVec2Input("origin", "y", e)}
              step="1"
            />
          </div>

          {/* Stroke width */}
          <div class="tool-option-row">
            <span class="tool-option-label">Width</span>
            <input
              type="range"
              min="0.5"
              max="20"
              step="0.5"
              value={selectedStroke()!.strokeWidth}
              onInput={(e) => {
                const val = parseFloat((e.target as HTMLInputElement).value);
                if (!isNaN(val)) updateProperty("strokeWidth", val);
              }}
            />
            <span class="tool-option-value">{selectedStroke()!.strokeWidth.toFixed(1)}</span>
          </div>

          {/* Color swatches */}
          <div class="tool-option-row">
            <span class="tool-option-label">Stroke</span>
            <div
              class="color-swatch-inline"
              style={{
                background: selectedStroke()!.strokeColorIndex === 0
                  ? "repeating-conic-gradient(#ccc 0% 25%, transparent 0% 50%) 50% / 8px 8px"
                  : strokeColorCSS(),
              }}
              title={`Stroke color #${selectedStroke()!.strokeColorIndex} — Click to set from active color`}
              onClick={() => handleColorSwatchClick("strokeColorIndex")}
            />
            <span class="tool-option-value">#{selectedStroke()!.strokeColorIndex}</span>
          </div>
          <div class="tool-option-row">
            <span class="tool-option-label">Fill</span>
            <div
              class="color-swatch-inline"
              style={{
                background: selectedStroke()!.fillColorIndex === 0
                  ? "repeating-conic-gradient(#ccc 0% 25%, transparent 0% 50%) 50% / 8px 8px"
                  : fillColorCSS(),
              }}
              title={`Fill color #${selectedStroke()!.fillColorIndex} — Click to set from active color`}
              onClick={() => handleColorSwatchClick("fillColorIndex")}
            />
            <span class="tool-option-value">#{selectedStroke()!.fillColorIndex}</span>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default SelectToolOptions;
