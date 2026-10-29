// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CodeMap from './CodeMap.svelte';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0, Level1, Level2 } from './types';

const block = (id: string, extra = {}) => ({
  id, label: id, group: 'L0', files: 3, symbols: 10, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 4 },
  blocks: [block('p/a', { complexity: { functions: 10, complex: 3 } }), block('p/b', { group: 'L1', complexity: null }), block('fold:L1', { group: 'L1', label: '+2 more', folded: ['p/c', 'p/d'] })],
  bands: [{ source: 'p/a', target: 'p/b', weight: 5, kinds: { calls: 5 }, direction: 'up' }],
  budget: { drawn: 3, modules: 4, bands: 1, edges: 7 },
};
const L1: Level1 = {
  level: 'file', module: 'p/a', dependsOn: [{ module: 'p/b', weight: 5, direction: 'up' }], usedBy: [],
  columns: [
    { role: 'code', files: [{ id: 'f1', path: 'a/x.rs', symbols: 3 }], more: 2, more_symbols: 4 },
    { role: 'test', files: [{ id: 'f2', path: 'a/tests/t.rs', symbols: 1 }], more: 0, more_symbols: 0 },
  ],
};
const L2: Level2 = {
  level: 'symbol', file: 'f1', path: 'a/x.rs', symbols: [{ id: 's1', name: 'scan', kind: 'function', line: 4 }],
  callers: [{ id: 'c1', name: 'main', file: 'm.rs', weight: 2 }], callersTotal: 9, callees: [], calleesTotal: 0,
};
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const ZONES = {
  meanDistance: 0.4, coverage: { points: 2, unplaceable: 1 },
  points: [
    { component: 'p/a', types: 2, abstractTypes: 0, ca: 3, ce: 0, abstractness: 0, instability: 0, distance: 1, zone: 'pain' as const },
    { component: 'p/b', types: 0, abstractTypes: 0, ca: 0, ce: 0, abstractness: 0, instability: 0, distance: 0, zone: null },
  ],
};
const port = (): CodeMapPort => ({
  level0: vi.fn(() => ok(L0)), module: vi.fn(() => ok(L1)), file: vi.fn(() => ok(L2)), zones: vi.fn(() => ok(ZONES)),
  cochange: vi.fn(() => ok({ hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }], pairs: 3, commits: 40, minTogether: 5 })),
});

let target: HTMLElement;
let app: Record<string, unknown> | null = null;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });
const render = (p: CodeMapPort) => {
  target = document.body.appendChild(document.createElement('div'));
  const state = new CodeMapState();
  app = mount(CodeMap, { target, props: { state, port: p } });
  return state;
};
const q = (sel: string) => target.querySelector(sel);
const settle = async () => { await new Promise((r) => setTimeout(r, 0)); flushSync(); };

describe('CodeMap', () => {
  it('draws no lens switch of its own — the host lists the lenses', async () => {
    render(port());
    await settle();
    expect(q('[aria-label="Lens"]')).toBeNull();
    expect(q('[data-testid^="codemap-lens-"]')).toBeNull();
  });

  it('opens on modules with the budget, and an upward band in red', async () => {
    render(port());
    await settle();
    expect(q('[data-testid="codemap-budget"]')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('drawing 3 blocks for 4 modules · 1 bands for 7 edges · 4 calls unplaced');
    expect(q('[data-testid="band-p/a>p/b"]')?.getAttribute('stroke')).toBe('var(--danger)');
    expect(q('[data-testid="codemap-level"]')?.textContent).toContain('L0 · modules');
  });

  it('drills module → files → symbols, and the breadcrumb comes back up', async () => {
    const p = port();
    render(p);
    await settle();
    (q('[data-testid="block-p/a"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    flushSync();
    (q('[data-testid="codemap-open"]') as HTMLButtonElement).click();
    await settle();
    expect(p.module).toHaveBeenCalledWith('p/a');
    expect(q('[data-testid="column-code"]')?.textContent).toContain('+2 more');
    (q('[data-testid="codemap-role-test"]') as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="column-test"]')).toBeNull();
    (q('[data-testid="file-a/x.rs"]') as HTMLButtonElement).click();
    await settle();
    expect(q('[data-testid="symbols-callers"]')?.textContent).toContain('+8 more');
    expect([...target.querySelectorAll('[data-testid="codemap-crumbs"] button')].map((b) => b.textContent)).toEqual([
      'workspace', 'p/a', 'a/x.rs',
    ]);
    ([...target.querySelectorAll('[data-testid="codemap-crumbs"] button')][0] as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="codemap-modules"]')).not.toBeNull();
  });

  it('a folded block lists its modules instead of opening', async () => {
    render(port());
    await settle();
    (q('[data-testid="block-fold:L1"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    flushSync();
    expect(q('[data-testid="codemap-inspector"]')?.textContent).toContain('p/c');
    expect(q('[data-testid="codemap-open"]')).toBeNull();
  });

  it('the Zones lens places components and names the furthest', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'zones');
    await settle();
    expect(q('[data-testid="zone-p/a"]')).not.toBeNull();
    expect(q('[data-testid="zone-p/b"]')).toBeNull();
    expect(q('[data-testid="zones-summary"]')?.textContent).toContain('1 components placed · mean distance 0.40 · pain 1');
    expect(q('[data-testid="zones-summary"]')?.textContent).toContain('1 without enough to place');
  });

  it('Hidden coupling draws co-change as dashed bands, a folded module to its block', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'coupling');
    await settle();
    const band = q('[data-testid="band-p/a>fold:L1"]');
    expect(band?.getAttribute('stroke-dasharray')).toBe('6 4');
    expect(q('[data-testid="band-p/a>p/b"]')).toBeNull();
    expect(q('[data-testid="coupling-summary"]')?.textContent).toContain('1 pairs change together with no dependency between them · from 40 commits');
  });

  it('the Wheel puts the modules on a ring and opens one on double-click', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'wheel');
    flushSync();
    expect(target.querySelectorAll('[data-testid^="wheel-node-"]').length).toBe(3);
    expect(q('[data-testid="wheel-band-p/a>p/b"]')).not.toBeNull();
    (q('[data-testid="wheel-node-p/a"]') as SVGElement).dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    await settle();
    expect(p.module).toHaveBeenCalledWith('p/a');
  });

  it('Complexity tints each block by its share over cyclomatic 10, and names the unmeasured', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'complexity');
    flushSync();
    expect(q('[data-testid="block-note-p/a"]')?.textContent?.trim()).toBe('30% over cx 10');
    expect(q('[data-testid="block-note-p/b"]')?.textContent?.trim()).toBe('not measured');
    expect(q('[data-testid="complexity-summary"]')?.textContent).toContain('3 of 10 functions over cyclomatic 10 · 1 of 3 blocks measured');
  });

  it('says why the map could not be read', async () => {
    render({ ...port(), level0: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Service Unavailable' })) });
    await settle();
    expect(q('[data-testid="codemap-error"]')?.textContent).toContain('Service Unavailable');
  });
});
