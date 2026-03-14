import type { Vec2, PathVertex, StrokeElement, Element } from "../lib/types";
import { vec2Distance } from "../lib/math";

/**
 * Test if a position is near any vertex, returning the vertex ID if within threshold.
 */
export function hitTestVertex(
  pos: Vec2,
  vertices: PathVertex[],
  threshold: number,
): string | null {
  for (const v of vertices) {
    if (vec2Distance(pos, v.pos) <= threshold) {
      return v.id;
    }
  }
  return null;
}

/**
 * Build a Path2D for a StrokeElement, applying its transform.
 */
function buildElementPath(element: StrokeElement): Path2D {
  const path = new Path2D();
  const verts = element.vertices;
  if (verts.length === 0) return path;

  path.moveTo(verts[0].pos.x, verts[0].pos.y);

  for (let i = 1; i < verts.length; i++) {
    const prev = verts[i - 1];
    const curr = verts[i];

    if (prev.cp1 && curr.cp2) {
      // Both have control points: cubic bezier
      path.bezierCurveTo(
        prev.cp1.x,
        prev.cp1.y,
        curr.cp2.x,
        curr.cp2.y,
        curr.pos.x,
        curr.pos.y,
      );
    } else {
      path.lineTo(curr.pos.x, curr.pos.y);
    }
  }

  if (element.closed && verts.length > 1) {
    const last = verts[verts.length - 1];
    const first = verts[0];
    if (last.cp1 && first.cp2) {
      path.bezierCurveTo(
        last.cp1.x,
        last.cp1.y,
        first.cp2.x,
        first.cp2.y,
        first.pos.x,
        first.pos.y,
      );
    } else {
      path.closePath();
    }
  }

  return path;
}

/**
 * Test if a position hits a StrokeElement's stroke or fill area.
 * Uses the canvas context's isPointInStroke/isPointInPath for accurate hit testing.
 */
export function hitTestElement(
  pos: Vec2,
  element: StrokeElement,
  ctx: CanvasRenderingContext2D,
): boolean {
  ctx.save();

  // Apply element transform
  ctx.translate(element.position.x, element.position.y);
  ctx.rotate(element.rotation);
  ctx.scale(element.scale.x, element.scale.y);
  ctx.translate(-element.origin.x, -element.origin.y);

  const path = buildElementPath(element);

  // Transform the test point into element-local space
  // We need the inverse transform, but since we set ctx transform we can use
  // isPointInStroke which respects the current transform
  ctx.lineWidth = Math.max(element.strokeWidth, 4); // minimum hit area

  const inStroke = ctx.isPointInStroke(path, pos.x, pos.y);
  const inFill = element.closed ? ctx.isPointInPath(path, pos.x, pos.y) : false;

  ctx.restore();

  return inStroke || inFill;
}

/**
 * Find a vertex on the same layer that could be a merge target.
 * Excludes vertices belonging to the current element being drawn.
 */
export function findMergeTarget(
  pos: Vec2,
  currentLayerElements: Element[],
  currentElementId: string | null,
  threshold: number,
): { elementId: string; vertexId: string } | null {
  for (const el of currentLayerElements) {
    if (el.type !== "stroke") continue;
    if (el.id === currentElementId) continue;

    for (const v of el.vertices) {
      if (vec2Distance(pos, v.pos) <= threshold) {
        return { elementId: el.id, vertexId: v.id };
      }
    }
  }
  return null;
}
