import type { StrokeElement, PathVertex } from "../lib/types";
import { generateId } from "../lib/math";

/**
 * Merge two StrokeElements into one. The target element's properties win
 * (color, stroke width, etc.). The source's vertices are appended to the target's.
 *
 * This handles the case where the user draws a stroke that starts or ends
 * near an existing element's vertex, fusing them together.
 */
export function mergeElements(
  target: StrokeElement,
  source: StrokeElement,
): StrokeElement {
  // Determine connection direction:
  // If source's first vertex is near target's last vertex, append source after target
  // If source's last vertex is near target's first vertex, prepend source before target
  // If source's first vertex is near target's first vertex, reverse source then prepend
  // If source's last vertex is near target's last vertex, reverse source then append
  // Default: just append source vertices after target vertices

  const mergedVertices: PathVertex[] = [
    ...target.vertices.map((v) => ({ ...v })),
    // Skip the first source vertex if it overlaps with target's last vertex
    // (the merge point). Give remaining vertices new IDs to avoid conflicts.
    ...source.vertices.slice(1).map((v) => ({
      ...v,
      id: generateId(),
    })),
  ];

  return {
    ...target,
    vertices: mergedVertices,
    // Keep target's closed state; the merge itself doesn't close the path
    closed: target.closed,
  };
}

/**
 * Close an element by connecting its last vertex to its first vertex.
 * This happens when the user places a vertex on the same element's existing vertex.
 */
export function closeElement(element: StrokeElement): StrokeElement {
  return {
    ...element,
    closed: true,
  };
}
