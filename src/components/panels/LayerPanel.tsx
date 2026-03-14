import { type Component, createMemo, createSignal, For, Show } from "solid-js";
import { getActiveSprite, updateSprite } from "@/stores/project";
import { editorStore, setActiveLayerId } from "@/stores/editor";
import { generateId } from "@/lib/math";
import { history } from "@/lib/history";
import {
  IconAdd,
  IconDelete,
  IconDuplicate,
  IconVisible,
  IconHidden,
  IconLock,
  IconUnlock,
  IconMirror,
  IconCombine,
} from "../../assets/icons";
import type { Layer, StrokeElement } from "@/lib/types";

const LayerPanel: Component = () => {
  const [dragFromIndex, setDragFromIndex] = createSignal<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = createSignal<number | null>(null);

  const selectedLayerId = createMemo(() => editorStore.activeLayerId);

  const layers = createMemo(() => {
    const sprite = getActiveSprite();
    return sprite?.layers ?? [];
  });

  const selectLayer = (layerId: string) => {
    setActiveLayerId(layerId);
  };

  const toggleVisibility = (layerIndex: number, e: MouseEvent) => {
    e.stopPropagation();
    const sprite = getActiveSprite();
    if (!sprite) return;
    const layer = sprite.layers[layerIndex];
    if (!layer) return;

    const oldVisible = layer.visible;
    history.execute({
      description: `Toggle layer "${layer.name}" visibility`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          s.layers[layerIndex].visible = !oldVisible;
        });
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          s.layers[layerIndex].visible = oldVisible;
        });
      },
    });
  };

  const toggleLock = (layerIndex: number, e: MouseEvent) => {
    e.stopPropagation();
    const sprite = getActiveSprite();
    if (!sprite) return;
    const layer = sprite.layers[layerIndex];
    if (!layer) return;

    const oldLocked = layer.locked;
    history.execute({
      description: `Toggle layer "${layer.name}" lock`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          s.layers[layerIndex].locked = !oldLocked;
        });
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          s.layers[layerIndex].locked = oldLocked;
        });
      },
    });
  };

  const addLayer = () => {
    const sprite = getActiveSprite();
    if (!sprite) return;

    const newLayer: Layer = {
      id: generateId(),
      name: `Layer ${sprite.layers.length + 1}`,
      visible: true,
      locked: false,
      elements: [],
    };

    history.execute({
      description: "Add layer",
      execute: () => {
        updateSprite(sprite.id, (s) => {
          s.layers.push(newLayer);
        });
        setActiveLayerId(newLayer.id);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          const idx = s.layers.findIndex((l) => l.id === newLayer.id);
          if (idx >= 0) s.layers.splice(idx, 1);
        });
      },
    });
  };

  const deleteLayer = () => {
    const sprite = getActiveSprite();
    const layerId = selectedLayerId();
    if (!sprite || !layerId) return;

    const layerIndex = sprite.layers.findIndex((l) => l.id === layerId);
    if (layerIndex < 0) return;

    // Deep copy for undo
    const removedLayer = JSON.parse(JSON.stringify(sprite.layers[layerIndex]));

    history.execute({
      description: `Delete layer "${removedLayer.name}"`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          const idx = s.layers.findIndex((l) => l.id === layerId);
          if (idx >= 0) s.layers.splice(idx, 1);
        });
        setActiveLayerId(null);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          s.layers.splice(layerIndex, 0, removedLayer);
        });
        setActiveLayerId(layerId);
      },
    });
  };

  const duplicateLayer = () => {
    const sprite = getActiveSprite();
    const layerId = selectedLayerId();
    if (!sprite || !layerId) return;

    const layerIndex = sprite.layers.findIndex((l) => l.id === layerId);
    if (layerIndex < 0) return;

    const original = sprite.layers[layerIndex];
    // Deep copy with new IDs
    const duplicated: Layer = JSON.parse(JSON.stringify(original));
    duplicated.id = generateId();
    duplicated.name = `${original.name} copy`;
    // Assign new IDs to all elements
    for (const el of duplicated.elements) {
      el.id = generateId();
      if (el.type === "stroke") {
        for (const v of el.vertices) {
          v.id = generateId();
        }
      }
    }

    history.execute({
      description: `Duplicate layer "${original.name}"`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          s.layers.splice(layerIndex + 1, 0, duplicated);
        });
        setActiveLayerId(duplicated.id);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          const idx = s.layers.findIndex((l) => l.id === duplicated.id);
          if (idx >= 0) s.layers.splice(idx, 1);
        });
        setActiveLayerId(layerId);
      },
    });
  };

  const mirrorLayer = () => {
    const sprite = getActiveSprite();
    const layerId = selectedLayerId();
    if (!sprite || !layerId) return;

    const layerIndex = sprite.layers.findIndex((l) => l.id === layerId);
    if (layerIndex < 0) return;

    const layer = sprite.layers[layerIndex];
    const strokeElements = layer.elements.filter((e) => e.type === "stroke") as StrokeElement[];
    if (strokeElements.length === 0) return;

    // Calculate bounding box center from all vertices
    let minX = Infinity;
    let maxX = -Infinity;
    for (const el of strokeElements) {
      for (const v of el.vertices) {
        const worldX = v.pos.x + el.position.x;
        if (worldX < minX) minX = worldX;
        if (worldX > maxX) maxX = worldX;
      }
    }
    const centerX = (minX + maxX) / 2;

    // Deep copy of original elements for undo
    const originalElements = JSON.parse(JSON.stringify(layer.elements));

    history.execute({
      description: `Mirror layer "${layer.name}" horizontally`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          const l = s.layers[layerIndex];
          for (const el of l.elements) {
            if (el.type === "stroke") {
              // Flip each vertex x around center
              for (const v of el.vertices) {
                const worldX = v.pos.x + el.position.x;
                const mirroredWorldX = 2 * centerX - worldX;
                v.pos.x = mirroredWorldX - el.position.x;

                // Flip control points if they exist
                if (v.cp1) {
                  const cp1WorldX = v.cp1.x + el.position.x;
                  v.cp1.x = 2 * centerX - cp1WorldX - el.position.x;
                }
                if (v.cp2) {
                  const cp2WorldX = v.cp2.x + el.position.x;
                  v.cp2.x = 2 * centerX - cp2WorldX - el.position.x;
                }
              }
            }
          }
        });
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          s.layers[layerIndex].elements = originalElements;
        });
      },
    });
  };

  const combineWithBelow = () => {
    const sprite = getActiveSprite();
    const layerId = selectedLayerId();
    if (!sprite || !layerId) return;

    const layerIndex = sprite.layers.findIndex((l) => l.id === layerId);
    if (layerIndex < 0 || layerIndex === 0) return; // Can't combine first layer (nothing below)

    const currentLayer = JSON.parse(JSON.stringify(sprite.layers[layerIndex]));
    const belowLayer = JSON.parse(JSON.stringify(sprite.layers[layerIndex - 1]));

    history.execute({
      description: `Combine "${currentLayer.name}" with "${belowLayer.name}"`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          // Merge elements from current layer into the layer below
          const below = s.layers[layerIndex - 1];
          const current = s.layers[layerIndex];
          below.elements = [...below.elements, ...current.elements];
          // Remove the current layer
          s.layers.splice(layerIndex, 1);
        });
        setActiveLayerId(belowLayer.id);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          // Restore the below layer to its original state
          s.layers[layerIndex - 1] = belowLayer;
          // Re-insert the current layer
          s.layers.splice(layerIndex, 0, currentLayer);
        });
        setActiveLayerId(layerId);
      },
    });
  };

  // --- Drag and drop reorder ---

  const handleDragStart = (index: number, e: DragEvent) => {
    setDragFromIndex(index);
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(index));
    }
  };

  const handleDragOver = (index: number, e: DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
    setDragOverIndex(index);
  };

  const handleDragLeave = () => {
    setDragOverIndex(null);
  };

  const handleDrop = (toIndex: number, e: DragEvent) => {
    e.preventDefault();
    const fromIndex = dragFromIndex();
    setDragFromIndex(null);
    setDragOverIndex(null);

    if (fromIndex === null || fromIndex === toIndex) return;

    const sprite = getActiveSprite();
    if (!sprite) return;

    const movedLayer = JSON.parse(JSON.stringify(sprite.layers[fromIndex]));

    history.execute({
      description: `Reorder layer "${movedLayer.name}"`,
      execute: () => {
        updateSprite(sprite.id, (s) => {
          const [removed] = s.layers.splice(fromIndex, 1);
          const insertAt = fromIndex < toIndex ? toIndex - 1 : toIndex;
          s.layers.splice(insertAt, 0, removed);
        });
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          // Reverse: find the moved layer and put it back
          const currentIdx = s.layers.findIndex((l) => l.id === movedLayer.id);
          if (currentIdx >= 0) {
            const [removed] = s.layers.splice(currentIdx, 1);
            s.layers.splice(fromIndex, 0, removed);
          }
        });
      },
    });
  };

  const handleDragEnd = () => {
    setDragFromIndex(null);
    setDragOverIndex(null);
  };

  const canCombine = createMemo(() => {
    const layerId = selectedLayerId();
    if (!layerId) return false;
    const sprite = getActiveSprite();
    if (!sprite) return false;
    const idx = sprite.layers.findIndex((l) => l.id === layerId);
    return idx > 0; // Must have a layer below
  });

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div
        class="panel-section"
        style={{ flex: "1", "min-height": "0", "overflow-y": "auto" }}
      >
        <Show
          when={layers().length > 0}
          fallback={<div class="placeholder">No layers</div>}
        >
          <For each={layers()}>
            {(layer, index) => (
              <div
                class={`layer-item ${selectedLayerId() === layer.id ? "selected" : ""} ${dragOverIndex() === index() ? "drag-over" : ""}`}
                onClick={() => selectLayer(layer.id)}
                draggable={true}
                onDragStart={(e) => handleDragStart(index(), e)}
                onDragOver={(e) => handleDragOver(index(), e)}
                onDragLeave={handleDragLeave}
                onDrop={(e) => handleDrop(index(), e)}
                onDragEnd={handleDragEnd}
              >
                <Show when={dragOverIndex() === index() && dragFromIndex() !== index()}>
                  <div class="drag-indicator" />
                </Show>
                <span class="layer-item-name">{layer.name}</span>
                <div class="layer-item-actions">
                  <button
                    class="icon-btn"
                    onClick={(e) => toggleVisibility(index(), e)}
                    title={layer.visible ? "Hide Layer" : "Show Layer"}
                  >
                    {layer.visible ? (
                      <IconVisible size={16} />
                    ) : (
                      <IconHidden size={16} />
                    )}
                  </button>
                  <button
                    class="icon-btn"
                    onClick={(e) => toggleLock(index(), e)}
                    title={layer.locked ? "Unlock Layer" : "Lock Layer"}
                  >
                    {layer.locked ? (
                      <IconLock size={16} />
                    ) : (
                      <IconUnlock size={16} />
                    )}
                  </button>
                </div>
              </div>
            )}
          </For>
        </Show>
      </div>

      <div class="layer-actions">
        <button class="icon-btn" onClick={addLayer} title="Add Layer">
          <IconAdd size={18} />
        </button>
        <button
          class="icon-btn"
          onClick={deleteLayer}
          disabled={!selectedLayerId()}
          title="Delete Layer"
        >
          <IconDelete size={18} />
        </button>
        <button
          class="icon-btn"
          onClick={duplicateLayer}
          disabled={!selectedLayerId()}
          title="Duplicate Layer"
        >
          <IconDuplicate size={18} />
        </button>
        <button
          class="icon-btn"
          onClick={mirrorLayer}
          disabled={!selectedLayerId()}
          title="Mirror Layer Horizontally"
        >
          <IconMirror size={18} />
        </button>
        <button
          class="icon-btn"
          onClick={combineWithBelow}
          disabled={!canCombine()}
          title="Combine with Layer Below"
        >
          <IconCombine size={18} />
        </button>
      </div>
    </div>
  );
};

export default LayerPanel;
