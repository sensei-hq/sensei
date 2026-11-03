import { describe, expect, it, vi } from 'vitest';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0, Level1, Level2 } from './types';

const L0 = (groupBy: 'layers' | 'folders' = 'layers'): Level0 => ({
  level: 'module', groupBy, cycles: [], coverage: { unplaced: 3 },
  blocks: [{ id: 'p/a', label: 'p/a', group: 'L0', files: 2, symbols: 9, hub: false, weight_in: 0, weight_out: 1, folded: [] }],
  bands: [], budget: { drawn: 1, modules: 1, bands: 0, edges: 1 },
});
const L1: Level1 = {
  level: 'file', module: 'p/a', dependsOn: [], usedBy: [],
  columns: [
    { role: 'code', files: [{ id: 'f1', path: 'a/x.rs', symbols: 3 }], more: 0, more_symbols: 0 },
    { role: 'test', files: [{ id: 'f2', path: 'a/tests/t.rs', symbols: 1 }], more: 0, more_symbols: 0 },
  ],
};
const L2: Level2 = {
  level: 'symbol', file: 'f1', path: 'a/x.rs', symbols: [], callers: [], callersTotal: 0, callees: [], calleesTotal: 0,
};
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
function port(over: Partial<CodeMapPort> = {}): CodeMapPort {
  return {
    level0: vi.fn((g) => ok(L0(g))), module: vi.fn(() => ok(L1)), file: vi.fn(() => ok(L2)),
    zones: vi.fn(() => ok({ points: [], meanDistance: 0, coverage: { points: 0, unplaceable: 0 } })),
    cochange: vi.fn(() => ok({ hidden: [], pairs: 0, commits: 12, minTogether: 5 })),
    complexity: vi.fn(() => ok({ total: { functions: 0, complex: 0, maxCyclo: 0, files: 0 }, modules: [], maintainability: null })),
    ...over,
  };
}

describe('CodeMapState', () => {
  it('reads the Complexity tree the first time the lens opens, and only then', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'structure');
    expect(p.complexity).not.toHaveBeenCalled();
    await s.showLens(p, 'complexity');
    await s.showLens(p, 'complexity');
    expect(p.complexity).toHaveBeenCalledOnce();
    expect(s.complexity?.total.files).toBe(0);
  });

  it('Zones opens on pain and uselessness', () => {
    expect(new CodeMapState().zoneFilter).toBe('issues');
  });

  it('keeps a zoom per diagram: zooming one does not zoom the next (user, 2026-11-02)', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'zones');
    s.zoom = 1.5;
    await s.showLens(p, 'matrix');
    expect(s.zoom).toBe(1);
    s.zoom = 0.75;
    await s.showLens(p, 'zones');
    expect(s.zoom).toBe(1.5);
    await s.showLens(p, 'matrix');
    expect(s.zoom).toBe(0.75);
  });

  it('opens on modules, and drills module → file with a breadcrumb', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    expect([s.level, s.l0?.blocks.length]).toEqual(['modules', 1]);
    await s.openModule(p, 'p/a');
    expect([s.level, s.crumbs.map((c) => c.label)]).toEqual(['files', ['workspace', 'p/a']]);
    await s.openFile(p, 'f1', 'a/x.rs');
    expect([s.level, s.crumbs.map((c) => c.label)]).toEqual(['symbols', ['workspace', 'p/a', 'a/x.rs']]);
    s.crumbTo(1);
    expect([s.level, s.crumbs.length, s.l2]).toEqual(['files', 2, null]);
    s.crumbTo(0);
    expect([s.level, s.crumbs.length]).toEqual(['modules', 1]);
  });

  it('keeps level, selection and lens when the grouping changes', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    s.select('p/a');
    s.lens = 'matrix';
    await s.setGroupBy(p, 'folders');
    expect(p.level0).toHaveBeenLastCalledWith('folders', null);
    expect([s.groupBy, s.selected, s.lens, s.level]).toEqual(['folders', 'p/a', 'matrix', 'modules']);
  });

  it('hides a role from the file columns when it is turned off', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    await s.openModule(p, 'p/a');
    expect(s.columns.map((c) => c.role)).toEqual(['code', 'test']);
    s.toggleRole('test');
    expect(s.columns.map((c) => c.role)).toEqual(['code']);
  });

  it('says why a read failed, and stays where it was', async () => {
    const s = new CodeMapState();
    const p = port({ module: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Service Unavailable' })) });
    await s.load(p);
    await s.openModule(p, 'p/a');
    expect([s.level, s.error]).toEqual(['modules', 'Service Unavailable']);
  });

  it('every lens leaves the files view and keeps the focus (user, 2026-11-03)', async () => {
    for (const lens of ['structure', 'wheel', 'cycles', 'matrix', 'zones', 'coupling', 'complexity'] as const) {
      const s = new CodeMapState();
      const p = port();
      await s.setFocus(p, { kind: 'module', id: 'p/a' });
      await s.openModule(p, 'p/a');
      await s.openFile(p, 'f1', 'a/x.rs');
      await s.showLens(p, lens);
      expect([s.lens, s.level, s.crumbs.length, s.l1, s.l2, s.focus], lens).toEqual([lens, 'modules', 1, null, null, { kind: 'module', id: 'p/a' }]);
    }
  });

  it('a focus re-reads the map narrowed to it; clearing it reads the whole map again', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    expect(p.level0).toHaveBeenLastCalledWith('layers', null);
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    expect(p.level0).toHaveBeenLastCalledWith('layers', { kind: 'module', id: 'p/a' });
    expect(s.focus).toEqual({ kind: 'module', id: 'p/a' });
    await s.showLens(p, 'cycles');
    expect(s.focus).toEqual({ kind: 'module', id: 'p/a' });
    await s.clearFocus(p);
    expect([s.focus, (p.level0 as ReturnType<typeof vi.fn>).mock.lastCall]).toEqual([null, ['layers', null]]);
  });

  it('focusing from the files view climbs back to the modules', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    await s.openModule(p, 'p/a');
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    expect([s.level, s.crumbs.length, s.l1]).toEqual(['modules', 1, null]);
  });

  it('a regrouping keeps a module focus, and drops a segment focus the new grouping has no group for', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    await s.setGroupBy(p, 'folders');
    expect(p.level0).toHaveBeenCalledWith('folders', { kind: 'module', id: 'p/a' });
    await s.setFocus(p, { kind: 'segment', id: 'L1' });
    await s.setGroupBy(p, 'layers');
    expect([s.focus, (p.level0 as ReturnType<typeof vi.fn>).mock.lastCall]).toEqual([null, ['layers', null]]);
  });

  it('reads zones once, when the Zones lens first opens', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'zones');
    await s.showLens(p, 'structure');
    await s.showLens(p, 'zones');
    expect([s.lens, s.zones?.meanDistance, p.zones]).toEqual(['zones', 0, p.zones]);
    expect(p.zones).toHaveBeenCalledTimes(1);
  });

  it('reads co-change once, when the Hidden coupling lens first opens', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'coupling');
    await s.showLens(p, 'coupling');
    expect([s.lens, s.cochange?.commits]).toEqual(['coupling', 12]);
    expect(p.cochange).toHaveBeenCalledTimes(1);
  });

  it('drops an answer that a later request overtook', async () => {
    const s = new CodeMapState();
    let release: (v: { ok: true; data: Level1 }) => void = () => {};
    const slow = new Promise<{ ok: true; data: Level1 }>((r) => (release = r));
    const p = port({ module: vi.fn().mockReturnValueOnce(slow).mockReturnValueOnce(ok({ ...L1, module: 'p/b' })) });
    await s.load(p);
    const first = s.openModule(p, 'p/a');
    await s.openModule(p, 'p/b');
    release({ ok: true, data: L1 });
    await first;
    expect(s.l1?.module).toBe('p/b');
  });

  it('a lens’s own read is kept when a different read finishes after it (user, 2026-11-03)', async () => {
    const s = new CodeMapState();
    let release: (v: { ok: true; data: { hidden: []; pairs: number; commits: number; minTogether: number } }) => void = () => {};
    const slow = new Promise<{ ok: true; data: { hidden: []; pairs: number; commits: number; minTogether: number } }>((r) => (release = r));
    const p = port({ cochange: vi.fn(() => slow) });
    await s.load(p);
    const coupling = s.showLens(p, 'coupling');
    await s.openModule(p, 'p/a');
    release({ ok: true, data: { hidden: [], pairs: 0, commits: 7, minTogether: 5 } });
    await coupling;
    expect([s.cochange?.commits, s.loading]).toEqual([7, false]);
  });

  it('keeps the whole map to find modules in while the lenses draw a focus', async () => {
    const s = new CodeMapState();
    const narrowed = { ...L0(), blocks: [] };
    const p = port({ level0: vi.fn((g, f) => ok(f ? narrowed : L0(g))) });
    await s.load(p);
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    expect([s.l0, s.whole?.blocks.length]).toEqual([narrowed, 1]);
  });

  it('a pick from the finder focuses a module, or a whole group as a segment', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.focusChoice(p, 'p/a');
    expect(s.focus).toEqual({ kind: 'module', id: 'p/a' });
    await s.focusChoice(p, 'group:L1');
    expect(s.focus).toEqual({ kind: 'segment', id: 'L1' });
  });

  // Review r1 F1: the finder is built from `whole`; a regroup under a focus must
  // re-read the unfocused map in the new grouping, or the finder offers the old
  // grouping's segments and picking one 404s.
  it('a regroup under a focus re-reads the whole map in the new grouping', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    await s.setGroupBy(p, 'folders');
    expect(p.level0).toHaveBeenCalledWith('folders', null);
    expect(s.whole?.groupBy).toBe('folders');
  });

  // Review r1 F2: each kind of read keeps its own error; a later success of
  // another kind must not clear it, or the lens sits on "Reading…" for good.
  it('a lens keeps its read’s failure when a different read succeeds after it', async () => {
    const s = new CodeMapState();
    const p = port({ cochange: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Gateway Timeout' })) });
    await s.load(p);
    await s.showLens(p, 'coupling');
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    expect(s.lensError).toBe('Gateway Timeout');
    await s.showLens(p, 'structure');
    expect(s.lensError).toBeNull();
  });

  // Review r1 F7: the state, not the component, narrows the lenses' data to the
  // focus, so any host reading the state gets the focused view.
  it('exposes the lenses’ data narrowed to the focus the map was read with', async () => {
    const s = new CodeMapState();
    const focused = { ...L0(), focus: { kind: 'module' as const, id: 'p/a', members: ['p/a'], neighbours: [] } };
    const p = port({
      level0: vi.fn((g, f) => ok(f ? focused : L0(g))),
      cochange: vi.fn(() => ok({ hidden: [{ a: 'p/a', b: 'p/x', together: 6, strength: 0.6 }, { a: 'p/y', b: 'p/x', together: 6, strength: 0.6 }], pairs: 2, commits: 9, minTogether: 5 })),
    });
    await s.showLens(p, 'coupling');
    expect(s.focusedCochange?.hidden.length).toBe(2);
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    expect(s.focusedCochange?.hidden.map((c) => c.b)).toEqual(['p/x']);
    expect(s.focusedCochange?.hidden.every((c) => c.a === 'p/a' || c.b === 'p/a')).toBe(true);
  });

  // Review r2 N1(b): a failed lens read is retried by showing the lens again,
  // and a success clears its error.
  it('Retry after a failed coupling read reads it again, and its success clears the error', async () => {
    const s = new CodeMapState();
    const cochange = vi.fn()
      .mockReturnValueOnce(Promise.resolve({ ok: false as const, error: 'Gateway Timeout' }))
      .mockReturnValueOnce(ok({ hidden: [], pairs: 0, commits: 9, minTogether: 5 }));
    const p = port({ cochange });
    await s.showLens(p, 'coupling');
    expect([s.lensError, s.dataError]).toEqual(['Gateway Timeout', 'Gateway Timeout']);
    expect(s.mapError).toBeNull();
    await s.showLens(p, 'coupling');
    expect(cochange).toHaveBeenCalledTimes(2);
    expect(s.lensError).toBeNull();
  });

  // Review r2 N2: a failed module read belongs to that attempt, not to every lens after it.
  it('a failed module read does not follow the reader to another lens', async () => {
    const s = new CodeMapState();
    const p = port({ module: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Not Found' })) });
    await s.load(p);
    await s.openModule(p, 'p/a');
    expect(s.lensError).toBe('Not Found');
    await s.showLens(p, 'zones');
    expect(s.lensError).toBeNull();
  });

  // Review r2 N3: if the whole map cannot be re-read after a regroup, the finder
  // falls back to the focused map (already in the new grouping), not the old one.
  it('a failed whole-map read after a regroup leaves no stale whole map', async () => {
    const s = new CodeMapState();
    const p = port({ level0: vi.fn((g, f) => (f ? ok(L0(g)) : g === 'folders' ? Promise.resolve({ ok: false as const, error: 'Bad Gateway' }) : ok(L0(g)))) });
    await s.load(p);
    await s.setFocus(p, { kind: 'module', id: 'p/a' });
    await s.setGroupBy(p, 'folders');
    expect(s.whole).toBeNull();
  });
});
