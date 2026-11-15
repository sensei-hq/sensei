// The Cycles lens (M17), pure: one ring per cycle — its members round a circle,
// the dependencies that close it as arrows, and the lightest of them marked as
// the one to cut. Nothing outside the cycles is drawn: that is the Structure lens.
import type { Level0 } from './types';

export interface RingNode {
  id: string;
  x: number;
  y: number;
}

export interface RingEdge {
  source: string;
  target: string;
  weight: number;
  /** The lightest dependency in the cycle: cutting it breaks the cycle cheapest. */
  cut: boolean;
  /** An SVG path, bent to the left of its direction so a pair both ways separates. */
  path: string;
  /** Where its weight is written: the middle of the bend. */
  label: { x: number; y: number };
}

export interface Ring {
  nodes: RingNode[];
  edges: RingEdge[];
  radius: number;
  /** The square the ring is drawn in, its labels' margin included. */
  size: number;
}

/** Room round the circle for the module names. */
const MARGIN = 90;
/** How far an arrow stops short of a node's centre. */
const STOP = 9;
/** How far an arrow bends, as a share of its length. */
const BEND = 0.18;

export function cycleRings(l0: Level0): Ring[] {
  return (l0.cycles as { members?: string[] }[])
    .map((c) => c.members ?? [])
    .filter((m) => m.length > 1)
    .map((members) => ring(l0, members));
}

function ring(l0: Level0, members: string[]): Ring {
  const radius = Math.max(70, 24 * members.length);
  const size = 2 * (radius + MARGIN);
  const c = size / 2;
  const nodes = members.map((id, i) => {
    const a = -Math.PI / 2 + (2 * Math.PI * i) / members.length;
    return { id, x: c + radius * Math.cos(a), y: c + radius * Math.sin(a) };
  });
  const at = new Map(nodes.map((n) => [n.id, n]));
  const inside = l0.bands.filter((b) => at.has(b.source) && at.has(b.target) && b.source !== b.target);
  const lightest = inside.reduce<(typeof inside)[number] | null>((min, b) => (!min || b.weight < min.weight ? b : min), null);
  const edges = inside.map((b) => {
    const [s, t] = [at.get(b.source)!, at.get(b.target)!];
    const { path, label } = bend(s, t);
    return { source: b.source, target: b.target, weight: b.weight, cut: b === lightest, path, label };
  });
  return { nodes, edges, radius, size };
}

function bend(s: RingNode, t: RingNode): { path: string; label: { x: number; y: number } } {
  const [dx, dy] = [t.x - s.x, t.y - s.y];
  const len = Math.hypot(dx, dy) || 1;
  const [ux, uy] = [dx / len, dy / len];
  // Left of the direction of travel: a → b and b → a bend to opposite sides.
  const [nx, ny] = [uy, -ux];
  const [x1, y1] = [s.x + ux * STOP, s.y + uy * STOP];
  const [x2, y2] = [t.x - ux * STOP, t.y - uy * STOP];
  const [cx, cy] = [(s.x + t.x) / 2 + nx * len * BEND, (s.y + t.y) / 2 + ny * len * BEND];
  const r = (v: number) => Math.round(v * 10) / 10;
  return {
    path: `M ${r(x1)} ${r(y1)} Q ${r(cx)} ${r(cy)} ${r(x2)} ${r(y2)}`,
    // The quadratic's midpoint: halfway between the chord's middle and the control.
    label: { x: r((x1 + x2) / 4 + cx / 2), y: r((y1 + y2) / 4 + cy / 2) },
  };
}
