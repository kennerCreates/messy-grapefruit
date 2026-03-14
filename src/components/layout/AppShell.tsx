import { createEffect, type Component } from "solid-js";
import { projectStore } from "@/stores/project";
import Toolbar from "./Toolbar";
import SidePanel from "./SidePanel";
import StatusBar from "./StatusBar";
import CanvasView from "../canvas/CanvasView";
import "../../styles/theme.css";

const AppShell: Component = () => {
  // Set data-theme attribute on the root element based on editor preferences
  createEffect(() => {
    const theme = projectStore.project?.editorPreferences?.theme ?? "dark";
    document.documentElement.setAttribute("data-theme", theme);
  });

  return (
    <div class="app-shell no-select">
      <Toolbar />
      <div class="canvas-area">
        <CanvasView />
      </div>
      <SidePanel />
      <StatusBar />
    </div>
  );
};

export default AppShell;
