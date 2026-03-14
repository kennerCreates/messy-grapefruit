import { type Component, createMemo, createSignal, For, Show } from "solid-js";
import { editorStore, setActiveColor } from "@/stores/editor";
import { paletteColors, colorToCSS, addColor, removeColor, updateColor } from "@/stores/palette";
import { IconAdd } from "../../assets/icons";
import { MAX_PALETTE_COLORS } from "@/lib/constants";
import type { PaletteColor } from "@/lib/types";
import LospecImporter from "../palette/LospecImporter";

function colorToHex(c: PaletteColor): string {
  const r = c.r.toString(16).padStart(2, "0");
  const g = c.g.toString(16).padStart(2, "0");
  const b = c.b.toString(16).padStart(2, "0");
  return `#${r}${g}${b}`;
}

function hexToColor(hex: string): PaletteColor | null {
  const match = hex.match(/^#?([0-9a-fA-F]{6})$/);
  if (!match) return null;
  const val = match[1];
  return {
    r: parseInt(val.substring(0, 2), 16),
    g: parseInt(val.substring(2, 4), 16),
    b: parseInt(val.substring(4, 6), 16),
    a: 255,
  };
}

const PalettePanel: Component = () => {
  const colors = createMemo(() => paletteColors());
  const activeColorIndex = createMemo(() => editorStore.activeColorIndex);
  const [editingIndex, setEditingIndex] = createSignal<number | null>(null);
  const [contextMenuIndex, setContextMenuIndex] = createSignal<number | null>(null);
  const [contextMenuPos, setContextMenuPos] = createSignal<{ x: number; y: number } | null>(null);

  const handleSwatchClick = (index: number) => {
    setActiveColor(index);
  };

  const handleSwatchContextMenu = (index: number, e: MouseEvent) => {
    e.preventDefault();
    if (index === 0) return; // Don't allow editing transparent
    setContextMenuIndex(index);
    setContextMenuPos({ x: e.clientX, y: e.clientY });
  };

  const closeContextMenu = () => {
    setContextMenuIndex(null);
    setContextMenuPos(null);
  };

  const handleEditColor = () => {
    const idx = contextMenuIndex();
    if (idx !== null) {
      setEditingIndex(idx);
    }
    closeContextMenu();
  };

  const handleDeleteColor = () => {
    const idx = contextMenuIndex();
    if (idx !== null && idx > 0) {
      removeColor(idx);
    }
    closeContextMenu();
  };

  const handleAddColor = () => {
    if (colors().length >= MAX_PALETTE_COLORS) return;
    addColor({ r: 180, g: 180, b: 180, a: 255 });
  };

  const handleColorChange = (channel: "r" | "g" | "b", value: number) => {
    const idx = editingIndex();
    if (idx === null) return;
    const current = colors()[idx];
    if (!current) return;
    updateColor(idx, { ...current, [channel]: Math.max(0, Math.min(255, value)) });
  };

  const handleHexChange = (hex: string) => {
    const idx = editingIndex();
    if (idx === null) return;
    const parsed = hexToColor(hex);
    if (parsed) {
      updateColor(idx, parsed);
    }
  };

  const atMax = createMemo(() => colors().length >= MAX_PALETTE_COLORS);

  return (
    <div
      style={{ display: "flex", "flex-direction": "column", height: "100%" }}
      onClick={() => closeContextMenu()}
    >
      <div
        class="palette-grid"
        style={{ flex: "1", "min-height": "0", "overflow-y": "auto" }}
      >
        <For each={colors()}>
          {(color, index) => (
            <div style={{ position: "relative", display: "inline-block" }}>
              <button
                class={`palette-swatch ${index() === activeColorIndex() ? "active" : ""} ${index() === 0 ? "transparent" : ""}`}
                style={index() > 0 ? { background: colorToCSS(color) } : undefined}
                onClick={() => handleSwatchClick(index())}
                onContextMenu={(e) => handleSwatchContextMenu(index(), e)}
                title={
                  index() === 0
                    ? "Transparent (index 0)"
                    : `Color ${index()} — rgb(${color.r}, ${color.g}, ${color.b})`
                }
              />
              <Show when={index() > 0}>
                <button
                  class="swatch-delete-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeColor(index());
                  }}
                  title="Delete color"
                >
                  x
                </button>
              </Show>
            </div>
          )}
        </For>
      </div>

      {/* Inline RGB Editor */}
      <Show when={editingIndex() !== null}>
        {(() => {
          const idx = editingIndex()!;
          const color = createMemo(() => colors()[idx]);
          return (
            <Show when={color()}>
              <div class="color-editor">
                <div class="color-editor-header">
                  <span>Edit Color #{idx}</span>
                  <button
                    class="icon-btn"
                    style={{ width: "20px", height: "20px", "font-size": "12px" }}
                    onClick={() => setEditingIndex(null)}
                    title="Close editor"
                  >
                    x
                  </button>
                </div>
                <div class="color-editor-preview" style={{ background: colorToCSS(color()!) }} />
                <div class="color-editor-row">
                  <span class="color-editor-label">R</span>
                  <input
                    type="range"
                    min="0"
                    max="255"
                    value={color()!.r}
                    onInput={(e) => handleColorChange("r", parseInt(e.currentTarget.value))}
                  />
                  <span class="color-editor-value">{color()!.r}</span>
                </div>
                <div class="color-editor-row">
                  <span class="color-editor-label">G</span>
                  <input
                    type="range"
                    min="0"
                    max="255"
                    value={color()!.g}
                    onInput={(e) => handleColorChange("g", parseInt(e.currentTarget.value))}
                  />
                  <span class="color-editor-value">{color()!.g}</span>
                </div>
                <div class="color-editor-row">
                  <span class="color-editor-label">B</span>
                  <input
                    type="range"
                    min="0"
                    max="255"
                    value={color()!.b}
                    onInput={(e) => handleColorChange("b", parseInt(e.currentTarget.value))}
                  />
                  <span class="color-editor-value">{color()!.b}</span>
                </div>
                <div class="color-editor-row">
                  <span class="color-editor-label">Hex</span>
                  <input
                    type="text"
                    class="color-editor-hex"
                    value={colorToHex(color()!)}
                    onInput={(e) => handleHexChange(e.currentTarget.value)}
                    maxLength={7}
                  />
                </div>
              </div>
            </Show>
          );
        })()}
      </Show>

      {/* Lospec Importer */}
      <LospecImporter />

      <div class="palette-footer">
        <span>
          {atMax() ? `(${MAX_PALETTE_COLORS}/${MAX_PALETTE_COLORS})` : `${colors().length}/${MAX_PALETTE_COLORS}`}
        </span>
        <button
          class="icon-btn"
          onClick={handleAddColor}
          disabled={atMax()}
          title={atMax() ? "Maximum colors reached" : "Add Color"}
        >
          <IconAdd size={16} />
        </button>
      </div>

      {/* Context menu */}
      <Show when={contextMenuPos() !== null && contextMenuIndex() !== null}>
        <div
          class="context-menu"
          style={{
            position: "fixed",
            left: `${contextMenuPos()!.x}px`,
            top: `${contextMenuPos()!.y}px`,
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button class="context-menu-item" onClick={handleEditColor}>
            Edit Color
          </button>
          <button class="context-menu-item" onClick={handleDeleteColor}>
            Delete Color
          </button>
        </div>
      </Show>
    </div>
  );
};

export default PalettePanel;
