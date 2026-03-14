import { type Component, onMount, onCleanup } from "solid-js";
import {
  editorStore,
  setEditorStore,
  screenToCanvas,
  zoomAt,
  toggleCurveMode,
} from "@/stores/editor";
import { createCanvasRenderer, type CanvasRenderer } from "@/engine/canvas";
import { history } from "@/lib/history";
import { setCursorPos } from "../layout/StatusBar";
import type { Vec2 } from "@/lib/types";

const CanvasView: Component = () => {
  let canvasRef!: HTMLCanvasElement;
  let containerRef!: HTMLDivElement;
  let renderer: CanvasRenderer | null = null;
  let resizeObserver: ResizeObserver | null = null;

  // Middle-mouse pan state
  let isPanning = false;
  let panStartScreen: Vec2 = { x: 0, y: 0 };
  let panStartViewport: Vec2 = { x: 0, y: 0 };

  /** Convert client coordinates to canvas-world coordinates using the store viewport */
  const clientToCanvas = (clientX: number, clientY: number): Vec2 => {
    const rect = canvasRef.getBoundingClientRect();
    return screenToCanvas(clientX - rect.left, clientY - rect.top);
  };

  const updateCanvasSize = () => {
    if (!canvasRef || !containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvasRef.width = rect.width * dpr;
    canvasRef.height = rect.height * dpr;
    canvasRef.style.width = `${rect.width}px`;
    canvasRef.style.height = `${rect.height}px`;
  };

  const handlePointerDown = (e: PointerEvent) => {
    // Middle mouse button for panning
    if (e.button === 1) {
      e.preventDefault();
      isPanning = true;
      panStartScreen = { x: e.clientX, y: e.clientY };
      panStartViewport = {
        x: editorStore.viewport.panX,
        y: editorStore.viewport.panY,
      };
      canvasRef.setPointerCapture(e.pointerId);
      return;
    }

    // Left mouse button — forward to active tool
    if (e.button === 0 && renderer) {
      const tool = renderer.getActiveTool();
      if (tool) {
        const canvasPos = clientToCanvas(e.clientX, e.clientY);
        tool.onPointerDown(e, canvasPos);
      }
    }
  };

  const handlePointerMove = (e: PointerEvent) => {
    // Update cursor canvas position for status bar
    const canvasPos = clientToCanvas(e.clientX, e.clientY);
    setCursorPos({ x: canvasPos.x, y: canvasPos.y });

    // Update cursor style based on active tool
    if (renderer && canvasRef) {
      canvasRef.style.cursor = isPanning ? "grabbing" : renderer.getCursor();
    }

    if (isPanning) {
      const dx = e.clientX - panStartScreen.x;
      const dy = e.clientY - panStartScreen.y;
      setEditorStore("viewport", "panX", panStartViewport.x + dx);
      setEditorStore("viewport", "panY", panStartViewport.y + dy);
      return;
    }

    // Forward to active tool
    if (renderer) {
      const tool = renderer.getActiveTool();
      if (tool) {
        tool.onPointerMove(e, canvasPos);
      }
    }
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (e.button === 1 && isPanning) {
      isPanning = false;
      canvasRef.releasePointerCapture(e.pointerId);
      return;
    }

    if (e.button === 0 && renderer) {
      const tool = renderer.getActiveTool();
      if (tool) {
        const canvasPos = clientToCanvas(e.clientX, e.clientY);
        tool.onPointerUp(e, canvasPos);
      }
    }
  };

  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    const rect = canvasRef.getBoundingClientRect();
    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;
    zoomAt(screenX, screenY, e.deltaY);
  };

  const handleDblClick = (e: MouseEvent) => {
    if (!renderer) return;
    const tool = renderer.getActiveTool();
    if (tool?.onDoubleClick) {
      const canvasPos = clientToCanvas(e.clientX, e.clientY);
      tool.onDoubleClick(e, canvasPos);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    // Ctrl+Z: Undo
    if (e.ctrlKey && e.key === "z") {
      e.preventDefault();
      history.undo();
      return;
    }

    // Ctrl+Y: Redo
    if (e.ctrlKey && e.key === "y") {
      e.preventDefault();
      history.redo();
      return;
    }

    // C: Toggle curve mode
    if ((e.key === "c" || e.key === "C") && !e.ctrlKey && !e.metaKey) {
      toggleCurveMode();
      return;
    }

    // Forward other keys to the active tool
    if (renderer) {
      const tool = renderer.getActiveTool();
      if (tool?.onKeyDown) {
        tool.onKeyDown(e);
      }
    }
  };

  onMount(() => {
    updateCanvasSize();

    // Initialize and start the canvas renderer
    renderer = createCanvasRenderer(canvasRef);
    renderer.start();

    // Set initial cursor based on active tool
    canvasRef.style.cursor = renderer.getCursor();

    // Resize observer
    resizeObserver = new ResizeObserver(() => {
      updateCanvasSize();
    });
    resizeObserver.observe(containerRef);

    // Keyboard events on the container
    containerRef.addEventListener("keydown", handleKeyDown);

    // Make the container focusable so it receives keyboard events
    containerRef.setAttribute("tabindex", "0");
    containerRef.focus();
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
      resizeObserver = null;
    }
    containerRef?.removeEventListener("keydown", handleKeyDown);
    if (renderer) {
      renderer.destroy();
      renderer = null;
    }
  });

  return (
    <div
      ref={containerRef!}
      style={{
        width: "100%",
        height: "100%",
        position: "relative",
        outline: "none",
      }}
    >
      <canvas
        ref={canvasRef!}
        style={{
          position: "absolute",
          top: "0",
          left: "0",
          cursor: "default",
        }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onWheel={handleWheel}
        onDblClick={handleDblClick}
        onContextMenu={(e) => e.preventDefault()}
      />
    </div>
  );
};

export default CanvasView;
