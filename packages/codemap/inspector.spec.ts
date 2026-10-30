import { describe, expect, it } from 'vitest';
import { inspect, workspaceStats } from './inspector';
import type { Block, Level0, Level1, Level2 } from './types';

const block = (id: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group: 'L1', package: 'sensei', files: 6, symbols: 40,
  roles: { code: 4, test: 2, docs: 0, config: 0, generated: 0 },
  hub: false, weight_in: 7, weight_out: 12, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers',
  cycles: [{ members: ['p/a', 'p/c'] }],
  coverage: { unplaced: 0 },
  blocks: [
    block('p/a', { complexity: { functions: 10, complex: 3 } }),
    block('p/b', { roles: { code: 2, test: 0, docs: 1, config: 0, generated: 0 } }),
    block('p/c'),
    block('p/log', { hub: true, weight_in: 90 }),
    block('fold:L1', { label: '+2 more', folded: ['p/x', 'p/y'] }),
  ],
  bands: [
    { source: 'p/a', target: 'p/b', weight: 5, kinds: {}, direction: 'up' },
    { source: 'p/a', target: 'p/c', weight: 3, kinds: {}, direction: 'skip' },
    { source: 'p/c', target: 'p/a', weight: 2, kinds: {}, direction: 'down' },
    { source: 'p/b', target: 'p/c', weight: 9, kinds: {}, direction: 'level' },
  ],
  budget: { drawn: 5, modules: 6, bands: 4, edges: 120 },
  libraries: { count: 14, top: ['serde', 'tokio'] },
};
const at0 = (selected: string | null) => inspect({ level: 'modules', l0: L0, l1: null, l2: null, selected });

describe('workspaceStats', () => {
  it('the numbers every diagram shares, and where the cycles are drawn', () => {
    expect(workspaceStats(L0).map((s) => [s.label, s.value, s.lens ?? null])).toEqual([
      ['Modules', '6', null], ['Symbols', '200', null], ['Edges', '120', null],
      ['Upward', '1', 'structure'], ['Skip a layer', '1', 'structure'], ['Cycles', '1', 'cycles'], ['Same layer · down', '1 · 1', null],
    ]);
  });
});

describe('inspect', () => {
  it('with nothing selected, summarises the workspace by direction', () => {
    const i = at0(null);
    expect([i.eyebrow, i.title]).toEqual(['Workspace', '6 modules']);
    expect(Object.fromEntries(i.stats.map((s) => [s.label, s.value]))).toEqual({
      Symbols: '200', Edges: '120', Upward: '1', 'Skip a layer': '1', Cycles: '1', 'Same layer · down': '1 · 1',
    });
  });

  it('a module: its files by role, what it leans on, and what to look at', () => {
    const i = at0('p/a');
    expect(i.eyebrow).toBe('Module · sensei · L1');
    expect(Object.fromEntries(i.stats.map((s) => [s.label, s.value]))).toEqual({
      Symbols: '40', Files: '6', 'Tests / code': '0.50', 'Over cx 10': '30%', 'Out · in': '12 · 7', Violations: '3',
    });
    expect(i.list.map((l) => [l.text, l.value])).toEqual([
      ['Upward call to p/b', '5'],
      ['Skips a layer to p/c', '3'],
      ['Cycle with p/c', ''],
      ['No docs in this module', ''],
    ]);
    expect(i.open).toBe('p/a');
  });

  it('a module with nothing to fix says so; unmeasured complexity is a dash, not 0%', () => {
    const i = at0('p/b');
    expect(i.stats.find((s) => s.label === 'Over cx 10')?.value).toBe('—');
    expect(i.list.map((l) => l.text)).toEqual(['No violations or cycles']);
  });

  it('a hub: its callers counted, and the rule that made it one', () => {
    const i = at0('p/log');
    expect([i.eyebrow, i.title, i.open]).toEqual(['Shared hub', 'p/log', null]);
    expect(i.stats.find((s) => s.label === 'Callers')?.value).toBe('90');
  });

  it('a fold lists what it holds', () => {
    const i = at0('fold:L1');
    expect([i.eyebrow, i.title]).toEqual(['Folded · L1', '2 smaller modules']);
    expect(i.list.map((l) => l.text)).toEqual(['p/x', 'p/y']);
  });

  it('the referenced libraries: how many, and the most used — never members', () => {
    const i = at0('libraries');
    expect([i.eyebrow, i.title]).toEqual(['Referenced libraries', '14 packages']);
    expect(i.list.map((l) => l.text)).toEqual(['serde', 'tokio']);
  });

  it('at L1, the open module, its violations read from what it depends on', () => {
    const l1: Level1 = {
      level: 'file', module: 'p/x', columns: [],
      dependsOn: [{ module: 'p/a', weight: 4, direction: 'up' }, { module: 'p/b', weight: 1, direction: 'down' }],
      usedBy: [],
    };
    const i = inspect({ level: 'files', l0: L0, l1, l2: null, selected: null });
    expect([i.eyebrow, i.title, i.open]).toEqual(['Module', 'p/x', null]);
    expect(i.list.map((l) => l.text)).toContain('Upward call to p/a');
  });

  it('at L2, the open file: its symbols, callers and callees', () => {
    const l2: Level2 = {
      level: 'symbol', file: 'f1', path: 'a/tests/x.rs', role: 'test', symbols: [{ id: 's', name: 'scan', kind: 'function', line: 1 }],
      callers: [], callersTotal: 9, callees: [], calleesTotal: 2,
    };
    const i = inspect({ level: 'symbols', l0: L0, l1: null, l2, selected: null });
    expect([i.eyebrow, i.title]).toEqual(['File · test', 'a/tests/x.rs']);
    expect(Object.fromEntries(i.stats.map((s) => [s.label, s.value]))).toEqual({ Symbols: '1', Callers: '9', Callees: '2' });
  });
});
