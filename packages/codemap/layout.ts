// The Code map's L0 layout (M17): one horizontal band per group, blocks in it
// left to right, wrapped to the width; hubs in a side lane; bands as curves
// between blocks. Pure — positions in, positions out — so both hosts draw the
// same picture and it is tested without a DOM.
import type { Band, Block, Direction, GroupBy, Level0 } from './types';

export const BLOCK_H = 36;
const GAP = 12;
const PAD = 16;
const LABEL_H = 20;
const LANE_W = 160;
const MIN_W = 72;
const MAX_W = 180;

export interface Placed {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  block: Block;
}

export interface GroupBandBox {
  id: string;
  y: number;
  height: number;
}

export interface DrawnBand {
  source: string;
  target: string;
  weight: number;
  width: number;
  direction: Direction | null;
  path: string;
}

export interface Layout0 {
  groups: GroupBandBox[];
  blocks: Map<string, Placed>;
  bands: DrawnBand[];
  hubLane: Placed[];
  laneX: number;
  width: number;
  height: number;
}

/** Groups in reading order: layers from the top (L0) down, anything unplaced
 *  last; folders largest first. */
export function groupOrder(blocks: Block[], groupBy: GroupBy): string[] {
  const size = new Map<string, number>();
  for (const b of blocks) size.set(b.group, (size.get(b.group) ?? 0) + b.symbols);
  const layer = (g: string) => (/^L\d+$/.test(g) ? Number(g.slice(1)) : Number.MAX_SAFE_INTEGER);
  return [...size.keys()].sort((a, b) =>
    groupBy === 'layers'
      ? layer(a) - layer(b) || a.localeCompare(b)
      : (size.get(b) ?? 0) - (size.get(a) ?? 0) || a.localeCompare(b),
  );
}

function blockWidth(b: Block): number {
  return Math.round(Math.min(MAX_W, Math.max(MIN_W, 24 + Math.sqrt(b.symbols) * 6)));
}

export function bandWidth(weight: number): number {
  return 1 + Math.log2(Math.max(1, weight));
}

export function layoutLevel0(data: Level0, opts: { width: number }): Layout0 {
  const width = Math.max(opts.width, MIN_W + LANE_W + 2 * PAD);
  const laneX = width - LANE_W;
  const blocks = new Map<string, Placed>();
  const groups: GroupBandBox[] = [];
  const members = data.blocks.filter((b) => !b.hub);
  let y = PAD;
  for (const group of groupOrder(members, data.groupBy)) {
    const top = y;
    let x = PAD;
    let rowY = y + LABEL_H;
    for (const b of members.filter((m) => m.group === group)) {
      const w = blockWidth(b);
      if (x + w > laneX - GAP && x > PAD) {
        x = PAD;
        rowY += BLOCK_H + GAP;
      }
      // Only a block wider than a whole row is narrowed; any other wraps.
      blocks.set(b.id, { id: b.id, x, y: rowY, w: Math.min(w, laneX - GAP - PAD), h: BLOCK_H, block: b });
      x += w + GAP;
    }
    const height = rowY + BLOCK_H + GAP - top;
    groups.push({ id: group, y: top, height });
    y = top + height + GAP;
  }
  const hubLane: Placed[] = [];
  let laneY = PAD + LABEL_H;
  for (const b of data.blocks.filter((m) => m.hub)) {
    const placed = { id: b.id, x: laneX + GAP, y: laneY, w: LANE_W - 2 * GAP, h: BLOCK_H, block: b };
    blocks.set(b.id, placed);
    hubLane.push(placed);
    laneY += BLOCK_H + GAP;
  }
  const bands: DrawnBand[] = data.bands
    .filter((b) => blocks.has(b.source) && blocks.has(b.target))
    .map((b) => {
      const s = blocks.get(b.source)!;
      const t = blocks.get(b.target)!;
      const [sx, sy] = [s.x + s.w / 2, s.y + (t.y >= s.y ? s.h : 0)];
      const [tx, ty] = [t.x + t.w / 2, t.y + (t.y >= s.y ? 0 : t.h)];
      const bend = Math.max(24, Math.abs(ty - sy) / 2);
      const dir = ty >= sy ? 1 : -1;
      return {
        source: b.source,
        target: b.target,
        weight: b.weight,
        width: bandWidth(b.weight),
        direction: b.direction,
        path: `M${sx},${sy} C${sx},${sy + dir * bend} ${tx},${ty - dir * bend} ${tx},${ty}`,
      };
    })
    .sort((a, b) => b.weight - a.weight);
  return { groups, blocks, bands, hubLane, laneX, width, height: Math.max(y, laneY) + PAD };
}
