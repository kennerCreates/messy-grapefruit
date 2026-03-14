import {
  type Component,
  createEffect,
  createMemo,
  Show,
  For,
} from "solid-js";
import {
  projectStore,
  setProject,
  setActiveTab,
  closeSpriteTab,
} from "@/stores/project";
import {
  FORMAT_VERSION,
  DEFAULT_GRID_SIZE,
} from "@/lib/constants";
import type { Project } from "@/lib/types";
import ProjectPage from "./pages/ProjectPage";
import EditorPage from "./pages/EditorPage";
import "./styles/global.css";
import "./styles/theme.css";

const App: Component = () => {
  const hasProject = createMemo(() => projectStore.project !== null);
  const activeTabId = createMemo(() => projectStore.activeTabId);

  /** Derive the list of open sprite tabs from the openSprites record */
  const openSpriteTabs = createMemo(() => {
    const sprites = projectStore.openSprites;
    return Object.values(sprites).map((s) => ({
      id: s.id,
      name: s.name,
    }));
  });

  /** Create a brand-new default project for testing */
  const createNewProject = () => {
    const project: Project = {
      name: "Untitled Project",
      formatVersion: FORMAT_VERSION,
      exportDir: "./export",
      palette: {
        name: "Default",
        colors: [
          { r: 0, g: 0, b: 0, a: 0 }, // index 0 = transparent
          { r: 251, g: 187, b: 173, a: 255 }, // warm text
          { r: 74, g: 122, b: 150, a: 255 }, // accent blue
          { r: 238, g: 134, b: 149, a: 255 }, // secondary pink
          { r: 51, g: 63, b: 88, a: 255 }, // panel dark
          { r: 41, g: 40, b: 49, a: 255 }, // bg dark
          { r: 255, g: 255, b: 255, a: 255 }, // white
          { r: 0, g: 0, b: 0, a: 255 }, // black
        ],
      },
      sprites: [],
      exportSettings: {
        mode: "bone",
        fps: 12,
        layout: "grid",
        trim: true,
        padding: 1,
      },
      editorPreferences: {
        theme: "dark",
        gridSize: DEFAULT_GRID_SIZE,
        gridMode: "standard",
        showGrid: true,
      },
    };

    setProject(project, "");
  };

  const handleCloseTab = (spriteId: string, e: MouseEvent) => {
    e.stopPropagation();
    closeSpriteTab(spriteId);
  };

  // Set theme on the root element
  createEffect(() => {
    const theme = projectStore.project?.editorPreferences?.theme ?? "dark";
    document.documentElement.setAttribute("data-theme", theme);
  });

  return (
    <Show
      when={hasProject()}
      fallback={<StartScreen onNewProject={createNewProject} />}
    >
      {/* App-level tab bar */}
      <div
        style={{ display: "flex", "flex-direction": "column", height: "100vh" }}
      >
        <div class="app-tabs">
          {/* Project overview tab (always first) */}
          <button
            class={`app-tab ${activeTabId() === null ? "active" : ""}`}
            onClick={() => setActiveTab(null)}
          >
            Project
          </button>

          {/* Open sprite tabs */}
          <For each={openSpriteTabs()}>
            {(tab) => (
              <button
                class={`app-tab ${activeTabId() === tab.id ? "active" : ""}`}
                onClick={() => setActiveTab(tab.id)}
              >
                <span>{tab.name}</span>
                <span
                  class="app-tab-close"
                  onClick={(e) => handleCloseTab(tab.id, e)}
                >
                  x
                </span>
              </button>
            )}
          </For>
        </div>

        {/* Tab content */}
        <div style={{ flex: "1", "min-height": "0", overflow: "hidden" }}>
          <Show when={activeTabId() === null}>
            <ProjectPage />
          </Show>
          <Show when={activeTabId() !== null}>
            <EditorPage />
          </Show>
        </div>
      </div>
    </Show>
  );
};

/** Start screen shown when no project is loaded */
const StartScreen: Component<{ onNewProject: () => void }> = (props) => {
  return (
    <div class="start-screen">
      <h1>Sprite Tool</h1>
      <p>SVG sprite drawing and animation for isometric games</p>
      <div class="start-screen-actions">
        <button class="btn-primary" onClick={props.onNewProject}>
          New Project
        </button>
        <button
          class="btn-secondary"
          onClick={() => {
            /* TODO: Tauri open dialog */
          }}
        >
          Open Project
        </button>
      </div>
    </div>
  );
};

export default App;
