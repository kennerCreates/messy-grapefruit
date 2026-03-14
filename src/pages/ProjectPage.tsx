import { type Component, For, Show } from "solid-js";
import { projectStore, openSpriteTab, setProjectStore } from "@/stores/project";
import { generateId } from "@/lib/math";
import {
  FORMAT_VERSION,
  DEFAULT_CANVAS_WIDTH,
  DEFAULT_CANVAS_HEIGHT,
} from "@/lib/constants";
import { IconAdd } from "../assets/icons";
import type { Sprite, Layer } from "@/lib/types";
import { produce } from "solid-js/store";

const ProjectPage: Component = () => {
  const handleNewSprite = () => {
    const spriteId = generateId();
    const defaultLayer: Layer = {
      id: generateId(),
      name: "Layer 1",
      visible: true,
      locked: false,
      elements: [],
    };

    const sprite: Sprite = {
      id: spriteId,
      name: "New Sprite",
      formatVersion: FORMAT_VERSION,
      canvasWidth: DEFAULT_CANVAS_WIDTH,
      canvasHeight: DEFAULT_CANVAS_HEIGHT,
      backgroundColorIndex: 0,
      layers: [defaultLayer],
      skins: [],
      animations: [],
    };

    // Add a sprite reference to the project
    setProjectStore(
      produce((state) => {
        if (!state.project) return;
        state.project.sprites.push({
          id: spriteId,
          filePath: `${sprite.name}.sprite`,
          position: { x: 0, y: 0 },
          rotation: 0,
          zOrder: state.project.sprites.length,
        });
      })
    );

    // Open the sprite in a new tab
    openSpriteTab(sprite);
  };

  return (
    <div style={{ padding: "24px", height: "100%", overflow: "auto" }}>
      <div
        style={{
          display: "flex",
          "align-items": "center",
          "justify-content": "space-between",
          "margin-bottom": "20px",
        }}
      >
        <h2 style={{ "font-size": "20px", "font-weight": "600" }}>
          {projectStore.project?.name ?? "Project Overview"}
        </h2>
        <button class="btn-primary" onClick={handleNewSprite}>
          <span
            style={{
              display: "flex",
              "align-items": "center",
              gap: "6px",
            }}
          >
            <IconAdd size={16} />
            New Sprite
          </span>
        </button>
      </div>

      <Show
        when={
          projectStore.project?.sprites &&
          projectStore.project.sprites.length > 0
        }
        fallback={
          <div class="placeholder" style={{ "min-height": "200px" }}>
            No sprites yet. Create one to get started.
          </div>
        }
      >
        <div
          style={{
            display: "grid",
            "grid-template-columns": "repeat(auto-fill, minmax(160px, 1fr))",
            gap: "16px",
          }}
        >
          <For each={projectStore.project!.sprites}>
            {(spriteRef) => {
              const spriteName =
                spriteRef.filePath
                  .split("/")
                  .pop()
                  ?.replace(".sprite", "") ?? spriteRef.id;

              return (
                <div
                  style={{
                    background: "var(--panels)",
                    "border-radius": "8px",
                    padding: "12px",
                    cursor: "pointer",
                    border: "1px solid var(--border)",
                    transition: "border-color 0.15s",
                  }}
                  onDblClick={() => {
                    // If the sprite is already open, just switch to its tab
                    const existingSprite =
                      projectStore.openSprites[spriteRef.id];
                    if (existingSprite) {
                      openSpriteTab(existingSprite);
                    }
                    // TODO: load from disk if not already open
                  }}
                  title="Double-click to open"
                >
                  <div
                    style={{
                      width: "100%",
                      "aspect-ratio": "1",
                      background: "var(--bg)",
                      "border-radius": "4px",
                      "margin-bottom": "8px",
                      display: "flex",
                      "align-items": "center",
                      "justify-content": "center",
                      "font-size": "11px",
                      opacity: "0.4",
                    }}
                  >
                    Preview
                  </div>
                  <div
                    style={{
                      "font-size": "12px",
                      "text-align": "center",
                      overflow: "hidden",
                      "text-overflow": "ellipsis",
                      "white-space": "nowrap",
                    }}
                  >
                    {spriteName}
                  </div>
                </div>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default ProjectPage;
