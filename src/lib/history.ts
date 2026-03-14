import { createSignal } from "solid-js";

export interface HistoryCommand {
  description: string;
  execute: () => void;
  undo: () => void;
}

export function createHistoryManager() {
  const undoStack: HistoryCommand[] = [];
  const redoStack: HistoryCommand[] = [];

  const [canUndo, setCanUndo] = createSignal(false);
  const [canRedo, setCanRedo] = createSignal(false);

  function updateSignals() {
    setCanUndo(undoStack.length > 0);
    setCanRedo(redoStack.length > 0);
  }

  function execute(cmd: HistoryCommand) {
    cmd.execute();
    undoStack.push(cmd);
    // Clear redo stack on new action
    redoStack.length = 0;
    updateSignals();
  }

  function undo() {
    const cmd = undoStack.pop();
    if (!cmd) return;
    cmd.undo();
    redoStack.push(cmd);
    updateSignals();
  }

  function redo() {
    const cmd = redoStack.pop();
    if (!cmd) return;
    cmd.execute();
    undoStack.push(cmd);
    updateSignals();
  }

  function clear() {
    undoStack.length = 0;
    redoStack.length = 0;
    updateSignals();
  }

  return {
    execute,
    undo,
    redo,
    clear,
    canUndo,
    canRedo,
  };
}

// Singleton history manager for the editor
export const history = createHistoryManager();
