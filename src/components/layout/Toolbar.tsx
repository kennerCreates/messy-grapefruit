import { type Component, createMemo } from "solid-js";
import { editorStore, setActiveTool } from "@/stores/editor";
import { projectStore, setProjectStore } from "@/stores/project";
import { history } from "@/lib/history";
import {
  IconLine,
  IconSelect,
  IconFill,
  IconEraser,
  IconUndo,
  IconRedo,
  IconSettings,
  IconSun,
  IconMoon,
} from "../../assets/icons";
import type { ToolType, ThemeMode } from "@/lib/types";

const Toolbar: Component = () => {
  const activeTool = createMemo(() => editorStore.activeTool);
  const theme = createMemo(
    () => projectStore.project?.editorPreferences?.theme ?? "dark"
  );

  const handleToolClick = (tool: ToolType) => {
    setActiveTool(tool);
  };

  const toggleTheme = () => {
    if (!projectStore.project) return;
    const newTheme: ThemeMode = theme() === "dark" ? "light" : "dark";
    setProjectStore("project", "editorPreferences", "theme", newTheme);
  };

  const handleUndo = () => {
    history.undo();
  };

  const handleRedo = () => {
    history.redo();
  };

  return (
    <div class="toolbar">
      {/* Left: Tool buttons */}
      <div class="toolbar-left">
        <div class="toolbar-group">
          <button
            class={`icon-btn ${activeTool() === "line" ? "active" : ""}`}
            onClick={() => handleToolClick("line")}
            title="Line Tool"
          >
            <IconLine />
          </button>
          <button
            class={`icon-btn ${activeTool() === "select" ? "active" : ""}`}
            onClick={() => handleToolClick("select")}
            title="Select Tool"
          >
            <IconSelect />
          </button>
          <button
            class={`icon-btn ${activeTool() === "fill" ? "active" : ""}`}
            onClick={() => handleToolClick("fill")}
            title="Fill Tool"
          >
            <IconFill />
          </button>
          <button
            class={`icon-btn ${activeTool() === "eraser" ? "active" : ""}`}
            onClick={() => handleToolClick("eraser")}
            title="Eraser Tool"
          >
            <IconEraser />
          </button>
        </div>
      </div>

      {/* Center: reserved for animation controls (Phase 3) */}
      <div class="toolbar-center" />

      {/* Right: Undo/Redo, Theme, Settings */}
      <div class="toolbar-right">
        <div class="toolbar-group">
          <button
            class="icon-btn"
            onClick={handleUndo}
            disabled={!history.canUndo()}
            title="Undo (Ctrl+Z)"
          >
            <IconUndo />
          </button>
          <button
            class="icon-btn"
            onClick={handleRedo}
            disabled={!history.canRedo()}
            title="Redo (Ctrl+Y)"
          >
            <IconRedo />
          </button>
        </div>
        <div class="toolbar-separator" />
        <button class="icon-btn" onClick={toggleTheme} title="Toggle Theme">
          {theme() === "dark" ? <IconSun /> : <IconMoon />}
        </button>
        <button class="icon-btn" title="Settings">
          <IconSettings />
        </button>
      </div>
    </div>
  );
};

export default Toolbar;
