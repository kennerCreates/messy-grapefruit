import { createStore, produce } from "solid-js/store";
import type { Project, Sprite } from "../lib/types";

export interface ProjectState {
  project: Project | null;
  projectPath: string | null;
  openSprites: Record<string, Sprite>; // id -> Sprite (SolidJS stores don't support Map)
  activeTabId: string | null; // null = project overview, string = sprite id
  dirty: boolean;
}

const initialState: ProjectState = {
  project: null,
  projectPath: null,
  openSprites: {},
  activeTabId: null,
  dirty: false,
};

export const [projectStore, setProjectStore] = createStore<ProjectState>(initialState);

// --- Actions ---

export function setProject(project: Project, path: string) {
  setProjectStore(
    produce((state) => {
      state.project = project;
      state.projectPath = path;
      state.dirty = false;
      state.openSprites = {};
      state.activeTabId = null;
    }),
  );
}

export function setActiveTab(tabId: string | null) {
  setProjectStore("activeTabId", tabId);
}

export function openSpriteTab(sprite: Sprite) {
  setProjectStore(
    produce((state) => {
      state.openSprites[sprite.id] = sprite;
      state.activeTabId = sprite.id;
    }),
  );
}

export function closeSpriteTab(spriteId: string) {
  setProjectStore(
    produce((state) => {
      delete state.openSprites[spriteId];
      if (state.activeTabId === spriteId) {
        // Switch to another open tab or project overview
        const openIds = Object.keys(state.openSprites);
        state.activeTabId = openIds.length > 0 ? openIds[0] : null;
      }
    }),
  );
}

export function updateSprite(spriteId: string, updater: (sprite: Sprite) => void) {
  setProjectStore(
    produce((state) => {
      const sprite = state.openSprites[spriteId];
      if (sprite) {
        updater(sprite);
        state.dirty = true;
      }
    }),
  );
}

export function setDirty(dirty: boolean) {
  setProjectStore("dirty", dirty);
}

/**
 * Get the currently active sprite (convenience accessor).
 */
export function getActiveSprite(): Sprite | null {
  const tabId = projectStore.activeTabId;
  if (!tabId) return null;
  return projectStore.openSprites[tabId] ?? null;
}
