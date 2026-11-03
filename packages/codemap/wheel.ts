// The Wheel lens (M17): hierarchical edge bundling. Modules sit on a ring in
// their group's arc; a band between two modules bends through their groups'
// anchors inside the ring, so dependencies between two groups bundle into one
// strand. Hubs stay off the ring, as they stay out of the L0 bands. Pure.
import { bandWidth, groupOrder } from './layout';
import type { Block, Direction, Level0 } from './types';

export interface WheelNode {
  id: string;
  x: number;
  y: number;
  angle: number;
  group: string;
  block: Block;
}

export interface WheelBand {
  source: string;
  target: string;
  weight: number;
  width: number;
  direction: Direction | null;
  path: string;
}

export interface Wheel {
  size: number;
  center: number;
  radius: number;
  nodes: Map<string, WheelNode>;
  arcs: { group: string; start: number; end: number }[];
  anchors: Map<string, { x: number; y: number }>;
  bands: WheelBand[];
}

/** Gap between group arcs, in node slots. */
const GROUP_GAP = 1;

export function layoutWheel(data: Level0, size: number): Wheel {
  const center = size / 2;
  const radius = size / 2 - 80;
  const ring = data.blocks.filter((b) => !b.hub);
  const groups = groupOrder(ring, data.groupBy);
  const slots = ring.length + groups.length * GROUP_GAP;
  const step = (2 * Math.PI) / Math.max(slots, 1);
  const at = (angle: number, r: number) => ({ x: center + r * Math.cos(angle), y: center + r * Math.sin(angle) });

  const nodes = new Map<string, WheelNode>();
  const arcs: Wheel['arcs'] = [];
  const anchors = new Map<string, { x: number; y: number }>();
  let slot = 0;
  for (const group of groups) {
    const members = ring.filter((b) => b.group === group);
    const start = slot * step - Math.PI / 2;
    for (const b of members) {
      const angle = slot * step - Math.PI / 2;
      nodes.set(b.id, { id: b.id, ...at(angle, radius), angle, group, block: b });
      slot += 1;
    }
    const end = (slot - 1) * step - Math.PI / 2;
    arcs.push({ group, start, end });
    anchors.set(group, at((start + end) / 2, radius * 0.45));
    slot += GROUP_GAP;
  }

  const bands: WheelBand[] = data.bands
    .filter((b) => nodes.has(b.source) && nodes.has(b.target))
    .map((b) => {
      const s = nodes.get(b.source)!;
      const t = nodes.get(b.target)!;
      const [sa, ta] = [anchors.get(s.group)!, anchors.get(t.group)!];
      // Within one group: a single bend toward its anchor. Across groups: one
      // cubic through both anchors — the bundle.
      const path =
        s.group === t.group
          ? `M${s.x},${s.y} Q${sa.x},${sa.y} ${t.x},${t.y}`
          : `M${s.x},${s.y} C${sa.x},${sa.y} ${ta.x},${ta.y} ${t.x},${t.y}`;
      return { source: b.source, target: b.target, weight: b.weight, width: bandWidth(b.weight), direction: b.direction, path };
    })
    .sort((a, b) => b.weight - a.weight);

  return { size, center, radius, nodes, arcs, anchors, bands };
}
