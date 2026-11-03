import { describe, expect, it } from 'vitest';
import { layoutWheel } from './wheel';
import type { Band, Block, Level0 } from './types';

const block = (id: string, group: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group, files: 1, symbols: 1, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const band = (source: string, target: string, weight = 1): Band => ({ source, target, weight, kinds: {}, direction: 'down' });
const l0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
  blocks: [block('a1', 'L0'), block('a2', 'L0'), block('b1', 'L1'), block('c1', 'L2'), block('h', 'L2', { hub: true })],
  bands: [band('a1', 'b1', 3), band('a2', 'a1'), band('b1', 'h')],
  budget: { drawn: 5, modules: 5, bands: 3, edges: 0 },
};

describe('layoutWheel', () => {
  const w = layoutWheel(l0, 400);
  const dist = (p: { x: number; y: number }) => Math.hypot(p.x - w.center, p.y - w.center);

  it('puts every module but a hub on the ring, each group on one contiguous arc', () => {
    expect([...w.nodes.keys()]).toEqual(['a1', 'a2', 'b1', 'c1']);
    for (const p of w.nodes.values()) expect(Math.abs(dist(p) - w.radius)).toBeLessThan(0.001);
    const order = [...w.nodes.values()].map((p) => p.group);
    expect(order).toEqual(['L0', 'L0', 'L1', 'L2']);
    expect(w.arcs.map((a) => a.group)).toEqual(['L0', 'L1', 'L2']);
  });

  it('bundles a band between two groups through their anchors, and skips bands to a hub', () => {
    expect(w.bands.map((b) => `${b.source}>${b.target}`)).toEqual(['a1>b1', 'a2>a1']);
    const across = w.bands[0];
    // Through both group anchors: the path names them, inside the ring.
    expect(across.path.match(/C/g)?.length).toBe(1);
    for (const a of [w.anchors.get('L0')!, w.anchors.get('L1')!]) expect(dist(a)).toBeLessThan(w.radius);
    expect(across.width).toBeGreaterThan(w.bands[1].width);
  });
});
