import type { Vec2, PathVertex } from "./types";

// --- Vec2 operations ---

export function vec2(x: number, y: number): Vec2 {
  return { x, y };
}

export function vec2Add(a: Vec2, b: Vec2): Vec2 {
  return { x: a.x + b.x, y: a.y + b.y };
}

export function vec2Sub(a: Vec2, b: Vec2): Vec2 {
  return { x: a.x - b.x, y: a.y - b.y };
}

export function vec2Scale(v: Vec2, s: number): Vec2 {
  return { x: v.x * s, y: v.y * s };
}

export function vec2Dot(a: Vec2, b: Vec2): number {
  return a.x * b.x + a.y * b.y;
}

export function vec2Length(v: Vec2): number {
  return Math.sqrt(v.x * v.x + v.y * v.y);
}

export function vec2Normalize(v: Vec2): Vec2 {
  const len = vec2Length(v);
  if (len === 0) return { x: 0, y: 0 };
  return { x: v.x / len, y: v.y / len };
}

export function vec2Distance(a: Vec2, b: Vec2): number {
  return vec2Length(vec2Sub(a, b));
}

export function vec2Lerp(a: Vec2, b: Vec2, t: number): Vec2 {
  return {
    x: a.x + (b.x - a.x) * t,
    y: a.y + (b.y - a.y) * t,
  };
}

export function vec2Rotate(v: Vec2, angle: number): Vec2 {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return {
    x: v.x * cos - v.y * sin,
    y: v.x * sin + v.y * cos,
  };
}

// --- Scalar utilities ---

export function clamp(val: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, val));
}

// --- Catmull-Rom to Bezier conversion ---

/**
 * Converts 4 Catmull-Rom points to cubic bezier control points for the p1->p2 segment.
 * Uses the standard Catmull-Rom to cubic Bezier conversion with alpha = 0.5 (uniform).
 */
export function catmullRomToBezier(
  p0: Vec2,
  p1: Vec2,
  p2: Vec2,
  p3: Vec2,
): { cp1: Vec2; cp2: Vec2 } {
  // Standard conversion: 1/6 factor for uniform Catmull-Rom
  const cp1: Vec2 = {
    x: p1.x + (p2.x - p0.x) / 6,
    y: p1.y + (p2.y - p0.y) / 6,
  };
  const cp2: Vec2 = {
    x: p2.x - (p3.x - p1.x) / 6,
    y: p2.y - (p3.y - p1.y) / 6,
  };
  return { cp1, cp2 };
}

/**
 * Auto-generate cp1/cp2 for each vertex using Catmull-Rom interpolation.
 * For open paths, phantom points are duplicated from endpoints (zero curvature at ends).
 * For closed paths, wrap around.
 */
export function generateAutoControlPoints(
  vertices: PathVertex[],
  closed: boolean,
): PathVertex[] {
  const n = vertices.length;
  if (n < 2) return vertices;

  return vertices.map((v, i) => {
    let p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2;

    p1 = v.pos;

    if (closed) {
      p0 = vertices[(i - 1 + n) % n].pos;
      p2 = vertices[(i + 1) % n].pos;
      p3 = vertices[(i + 2) % n].pos;
    } else {
      // Open path: duplicate endpoints as phantom points for zero curvature
      p0 = i > 0 ? vertices[i - 1].pos : v.pos;
      p2 = i < n - 1 ? vertices[i + 1].pos : v.pos;
      p3 = i < n - 2 ? vertices[i + 2].pos : p2;
    }

    // For vertex i, we need:
    // cp2 of vertex i (incoming handle): from the segment (i-1) -> i
    // cp1 of vertex i (outgoing handle): from the segment i -> (i+1)

    // Outgoing handle (cp1): tangent at p1 in the p0-p1-p2 segment
    const cp1: Vec2 = {
      x: p1.x + (p2.x - p0.x) / 6,
      y: p1.y + (p2.y - p0.y) / 6,
    };

    // Incoming handle (cp2): tangent at p1 in the prev-p1 segment
    // This is the mirror: p1 - (p2 - p0) / 6
    const cp2: Vec2 = {
      x: p1.x - (p2.x - p0.x) / 6,
      y: p1.y - (p2.y - p0.y) / 6,
    };

    return {
      ...v,
      cp1,
      cp2,
    };
  });
}

// --- ID generation ---

export function generateId(): string {
  return crypto.randomUUID();
}
