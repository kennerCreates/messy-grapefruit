import { type Component, createMemo, createSignal, For, Show } from "solid-js";
import { getActiveSprite, updateSprite } from "@/stores/project";
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
} from "../../assets/icons";
import type { Layer } from "@/lib/types";

const LayerPanel: Component = () => {
  const [selectedLayerId, setSelectedLayerId] = createSignal<string | null>(null);

  const layers = createMemo(() => {
    const sprite = getActiveSprite();
    return sprite?.layers ?? [];
  });

  const selectLayer = (layerId: string) => {
    setSelectedLayerId(layerId);
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
        setSelectedLayerId(newLayer.id);
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
        setSelectedLayerId(null);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          s.layers.splice(layerIndex, 0, removedLayer);
        });
        setSelectedLayerId(layerId);
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
        setSelectedLayerId(duplicated.id);
      },
      undo: () => {
        updateSprite(sprite.id, (s) => {
          const idx = s.layers.findIndex((l) => l.id === duplicated.id);
          if (idx >= 0) s.layers.splice(idx, 1);
        });
        setSelectedLayerId(layerId);
      },
    });
  };

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
                class={`layer-item ${selectedLayerId() === layer.id ? "selected" : ""}`}
                onClick={() => selectLayer(layer.id)}
              >
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
      </div>
    </div>
  );
};

export default LayerPanel;
