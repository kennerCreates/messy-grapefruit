import { type Component, createMemo, createSignal } from "solid-js";
import { editorStore } from "@/stores/editor";
import { projectStore } from "@/stores/project";

const TOOL_LABELS: Record<string, string> = {
  line: "Line",
  select: "Select",
  fill: "Fill",
  eraser: "Eraser",
};

/**
 * StatusBar uses a module-level signal for cursor position.
 * The CanvasView updates this via the exported setter.
 */
const [cursorPos, setCursorPos] = createSignal({ x: 0, y: 0 });
export { setCursorPos };

const StatusBar: Component = () => {
  const toolName = createMemo(() => TOOL_LABELS[editorStore.activeTool] ?? "Unknown");
  const zoom = createMemo(() => {
    const z = editorStore.viewport?.zoom ?? 1;
    return Math.round(z * 100);
  });
  const gridMode = createMemo(
    () => projectStore.project?.editorPreferences?.gridMode ?? "standard"
  );
  const curveLabel = createMemo(() => (editorStore.curveMode ? "Curve" : "Straight"));

  return (
    <div class="status-bar">
      <span>{toolName()}</span>
      <span>
        X: {cursorPos().x.toFixed(1)} Y: {cursorPos().y.toFixed(1)}
      </span>
      <span>{zoom()}%</span>
      <span>Grid: {gridMode()}</span>
      <span>{curveLabel()}</span>
    </div>
  );
};

export default StatusBar;
