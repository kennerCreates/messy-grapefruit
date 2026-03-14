import { type Component, createSignal, createMemo, Switch, Match } from "solid-js";
import { editorStore } from "@/stores/editor";
import LayerPanel from "../panels/LayerPanel";
import PalettePanel from "../panels/PalettePanel";
import LineToolOptions from "../sidebar/LineToolOptions";
import SelectToolOptions from "../sidebar/SelectToolOptions";
import FillToolOptions from "../sidebar/FillToolOptions";
import EraserToolOptions from "../sidebar/EraserToolOptions";

type SidebarTab = "layers" | "palette";

const SidePanel: Component = () => {
  const [activeTab, setActiveTab] = createSignal<SidebarTab>("layers");
  const activeTool = createMemo(() => editorStore.activeTool);

  return (
    <div class="sidebar">
      {/* Top zone: context-sensitive tool options */}
      <div class="sidebar-top">
        <Switch fallback={<div class="placeholder">No options</div>}>
          <Match when={activeTool() === "line"}>
            <LineToolOptions />
          </Match>
          <Match when={activeTool() === "select"}>
            <SelectToolOptions />
          </Match>
          <Match when={activeTool() === "fill"}>
            <FillToolOptions />
          </Match>
          <Match when={activeTool() === "eraser"}>
            <EraserToolOptions />
          </Match>
        </Switch>
      </div>

      {/* Bottom zone: fixed tabs */}
      <div class="sidebar-bottom">
        <div class="tab-bar">
          <button
            class={`tab ${activeTab() === "layers" ? "active" : ""}`}
            onClick={() => setActiveTab("layers")}
          >
            Layers
          </button>
          <button
            class={`tab ${activeTab() === "palette" ? "active" : ""}`}
            onClick={() => setActiveTab("palette")}
          >
            Palette
          </button>
        </div>

        <div class="panel-content">
          <Switch>
            <Match when={activeTab() === "layers"}>
              <LayerPanel />
            </Match>
            <Match when={activeTab() === "palette"}>
              <PalettePanel />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default SidePanel;
