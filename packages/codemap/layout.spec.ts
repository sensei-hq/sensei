import { describe, expect, it } from 'vitest';
import { layoutLevel0, groupOrder } from './layout';
import type { Block, Band, Level0 } from './types';

const block = (id: string, group: string, symbols: number, extra: Partial<Block> = {}): Block => ({
  id, label: id, group, files: 1, symbols, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const band = (source: string, target: string, weight = 1): Band => ({
  source, target, weight, kinds: { calls: weight }, direction: 'down',
});
const l0 = (blocks: Block[], bands: Band[], groupBy: 'layers' | 'folders' = 'layers'): Level0 => ({
  level: 'module', groupBy, blocks, bands, cycles: [], coverage: { unplaced: 0 },
  budget: { drawn: blocks.length, modules: blocks.length, bands: bands.length, edges: 0 },
});

describe('groupOrder', () => {
  it('reads layers top-down, unplaced last; folders by size, largest first', () => {
    const blocks = [block('a', 'L2', 1), block('b', 'L0', 1), block('c', 'unplaced', 1), block('d', 'L10', 1)];
    expect(groupOrder(blocks, 'layers')).toEqual(['L0', 'L2', 'L10', 'unplaced']);
    expect(groupOrder([block('a', 'small', 1), block('b', 'big', 9), block('c', 'big', 1)], 'folders')).toEqual(['big', 'small']);
  });
});

describe('layoutLevel0', () => {
  const blocks = [
    ...Array.from({ length: 12 }, (_, i) => block(`a${i}`, 'L0', 100 - i)),
    block('b0', 'L1', 50),
    block('log', 'L2', 5, { hub: true }),
  ];
  const layout = layoutLevel0(l0(blocks, [band('a0', 'b0', 4), band('a1', 'b0', 1)]), { width: 900 });

  it('lays each group out as a band, in order, top to bottom', () => {
    expect(layout.groups.map((g) => g.id)).toEqual(['L0', 'L1']);
    expect(layout.groups[0].y + layout.groups[0].height).toBeLessThanOrEqual(layout.groups[1].y);
    const a0 = layout.blocks.get('a0')!;
    expect(a0.y).toBeGreaterThanOrEqual(layout.groups[0].y);
    expect(a0.y + a0.h).toBeLessThanOrEqual(layout.groups[0].y + layout.groups[0].height);
  });

  it('wraps a crowded group within the width, with no block overlapping another', () => {
    const placed = [...layout.blocks.values()];
    for (const p of placed) {
      expect(p.x + p.w).toBeLessThanOrEqual(900);
      expect(p.w, `${p.id} keeps its width`).toBeGreaterThanOrEqual(72);
    }
    for (let i = 0; i < placed.length; i++)
      for (let j = i + 1; j < placed.length; j++) {
        const [p, q] = [placed[i], placed[j]];
        const apart = p.x + p.w <= q.x || q.x + q.w <= p.x || p.y + p.h <= q.y || q.y + q.h <= p.y;
        expect(apart, `${p.id} overlaps ${q.id}`).toBe(true);
      }
  });

  it('puts a hub in the side lane, outside every group band', () => {
    const hub = layout.blocks.get('log')!;
    expect(layout.hubLane.map((h) => h.id)).toEqual(['log']);
    expect(hub.x).toBeGreaterThanOrEqual(layout.laneX);
    for (const p of layout.blocks.values()) if (p.id !== 'log') expect(p.x + p.w).toBeLessThanOrEqual(layout.laneX);
  });

  it('draws each band between its blocks, heavier wider', () => {
    expect(layout.bands).toHaveLength(2);
    const [heavy, light] = layout.bands;
    expect(heavy.width).toBeGreaterThan(light.width);
    expect(heavy.path.startsWith('M')).toBe(true);
  });
});
