// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CodeMap from './CodeMap.svelte';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0, Level1, Level2 } from './types';

const roles = { code: 2, test: 1, docs: 0, config: 0, generated: 0 };
const block = (id: string, extra = {}) => ({
  id, label: id, group: 'L0', package: 'p', files: 3, symbols: 10, roles, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 4 },
  blocks: [block('p/a', { complexity: { functions: 10, complex: 3 } }), block('p/b', { group: 'L1', complexity: null }), block('fold:L1', { group: 'L1', label: '+2 more', folded: ['p/c', 'p/d'] })],
  bands: [
    { source: 'p/a', target: 'p/b', weight: 5, kinds: { calls: 5 }, direction: 'up' },
    { source: 'p/b', target: 'fold:L1', weight: 12, kinds: { calls: 12 }, direction: 'down' },
  ],
  budget: { drawn: 3, modules: 4, bands: 2, edges: 7 },
  libraries: { count: 14, top: ['serde', 'tokio'] },
};
const L1: Level1 = {
  level: 'file', module: 'p/a', dependsOn: [{ module: 'p/b', weight: 5, direction: 'up' }], usedBy: [],
  columns: [
    { role: 'code', files: [{ id: 'f1', path: 'a/x.rs', symbols: 3 }], more: 2, more_symbols: 4 },
    { role: 'test', files: [{ id: 'f2', path: 'a/tests/t.rs', symbols: 1 }], more: 0, more_symbols: 0 },
  ],
};
const L2: Level2 = {
  level: 'symbol', file: 'f1', path: 'a/x.rs', role: 'code', symbols: [{ id: 's1', name: 'scan', kind: 'function', line: 4 }],
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
  complexity: vi.fn(() => ok({
    total: { functions: 10, complex: 3, maxCyclo: 24, files: 1 },
    maintainability: null,
    modules: [{ id: 'p/a', label: 'p/a', kind: 'module', functions: 10, complex: 3, maxCyclo: 24, files: 1,
      children: [{ id: 'a/x.rs', label: 'x.rs', kind: 'file', functions: 10, complex: 3, maxCyclo: 24, files: 1, children: [] }] }],
  })),
});

let target: HTMLElement;
let app: Record<string, unknown> | null = null;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });
const render = (p: CodeMapPort, extra: Record<string, unknown> = {}) => {
  target = document.body.appendChild(document.createElement('div'));
  const state = new CodeMapState();
  app = mount(CodeMap, { target, props: { state, port: p, ...extra } });
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
    expect(q('[data-testid="codemap-budget"]')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('drawing 3 of 4 modules · 2 bands for 7 edges · 30 symbols · 4 calls unplaced');
    expect(q('[data-testid="band-p/a>p/b"]')?.getAttribute('stroke')).toBe('var(--danger)');
    expect(q('[data-testid="codemap-level"]')?.textContent).toContain('L0 · modules');
  });

  it('drills module → files → symbols, and the breadcrumb comes back up', async () => {
    const p = port();
    render(p);
    await settle();
    // One click focuses a module (user, 2026-11-03); its files open from the side panel.
    (q('[data-testid="block-p/a"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(p.level0).toHaveBeenLastCalledWith('layers', { kind: 'module', id: 'p/a' });
    expect(p.module).not.toHaveBeenCalled();
    (q('[data-testid="codemap-open"]') as HTMLButtonElement).click();
    await settle();
    expect(p.module).toHaveBeenCalledWith('p/a');
    expect(q('[data-testid="inspector-title"]')?.textContent).toBe('p/a');
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

  it('Hidden coupling draws the coupled pairs, not the block map, under a summary', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'coupling');
    await settle();
    expect(q('[data-testid="codemap-coupling"]')).not.toBeNull();
    expect(q('[data-testid="codemap-modules"]')).toBeNull();
    expect(target.querySelectorAll('[data-testid="codemap-coupling"] [data-pair]').length).toBe(1);
    expect(q('[data-testid="coupling-summary"]')?.textContent).toContain('1 pairs change together with no dependency between them · from 40 commits');
  });

  it('the Wheel puts the modules on a ring and focuses one on a click', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'wheel');
    flushSync();
    expect(target.querySelectorAll('[data-testid^="wheel-node-"]').length).toBe(3);
    expect(q('[data-testid="wheel-band-p/a>p/b"]')).not.toBeNull();
    (q('[data-testid="wheel-node-p/a"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(p.level0).toHaveBeenLastCalledWith('layers', { kind: 'module', id: 'p/a' });
    expect([map.focus, map.level, map.lens]).toEqual([{ kind: 'module', id: 'p/a' }, 'modules', 'wheel']);
  });

  it('picking a module on Hidden coupling focuses it and keeps drawing the lens', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'coupling');
    await settle();
    const pick = target.querySelector('[data-testid="codemap-coupling"] [aria-label^="Focus"]') as HTMLElement;
    pick.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(map.focus?.kind).toBe('module');
    expect([map.level, q('[data-testid="codemap-coupling"]') !== null]).toEqual(['modules', true]);
    expect(p.module).not.toHaveBeenCalled();
  });

  it('Complexity draws the tree, not the block map, and says what to act on from it', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'complexity');
    await settle();
    expect(q('[data-testid="codemap-complexity"] [data-tree]')).not.toBeNull();
    expect(q('[data-testid="codemap-modules"]')).toBeNull();
    expect(q('[data-testid="complexity-overall"]')?.textContent).toContain('3 of 10 functions over cyclomatic 10');
    expect(q('[data-testid="codemap-takeaways"]')?.textContent).toContain('1 file with functions over cyclomatic 10');
  });

  it('with nothing selected, the side panel says what to act on', async () => {
    const p = port();
    render(p);
    await settle();
    expect(q('[data-testid="codemap-takeaways"]')?.textContent).toContain('1 upward dependency');
  });

  it('Zones has no grouping or role filter — they would change nothing — but filters by zone', async () => {
    const p = port();
    const map = render(p);
    await settle();
    void map.showLens(p, 'zones');
    await settle();
    expect([q('[data-testid="codemap-group"]'), q('[data-testid="codemap-role-test"]')]).toEqual([null, null]);
    // The zone filter is a control, with the others.
    expect(q('[data-testid="codemap-controls"] [data-testid="codemap-zone"] [data-toggle]')).not.toBeNull();
    (q('[data-testid="codemap-zone"] [aria-label="Uselessness"]') as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="zones-furthest"]')?.textContent).not.toContain('p/a');
    (q('[data-testid="codemap-zone"] [aria-label="Pain"]') as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="zones-furthest"]')?.textContent).toContain('p/a');
    expect(q('[data-testid="zone-p/a"]')?.getAttribute('fill-opacity')).toBe('0.9');
  });

  it('Zones draws its frame before the data arrives, and the dots into it', async () => {
    let resolve!: (z: typeof ZONES) => void;
    const p = { ...port(), zones: vi.fn(() => new Promise<{ ok: true; data: typeof ZONES }>((r) => (resolve = (z) => r({ ok: true, data: z })))) };
    const map = render(p);
    await settle();
    void map.showLens(p, 'zones');
    await settle();
    expect(q('[data-testid="codemap-zones"] svg rect')).not.toBeNull();
    expect(q('[data-testid="zone-p/a"]')).toBeNull();
    resolve(ZONES);
    await settle();
    expect(q('[data-testid="zone-p/a"]')).not.toBeNull();
  });

  it('violations & cycles keeps the upward band; a minimum weight drops the light ones', async () => {
    const p = port();
    const map = render(p);
    await settle();
    map.edgeMode = 'violations';
    flushSync();
    expect([q('[data-testid="band-p/a>p/b"]'), q('[data-testid="band-p/b>fold:L1"]')].map(Boolean)).toEqual([true, false]);
    map.edgeMode = 'all';
    map.minWeight = 10;
    flushSync();
    expect([q('[data-testid="band-p/a>p/b"]'), q('[data-testid="band-p/b>fold:L1"]')].map(Boolean)).toEqual([false, true]);
  });

  it('colour by direction tints a down band; without it only upward is coloured', async () => {
    const map = render(port());
    await settle();
    expect(q('[data-testid="legend"]')?.textContent).toContain('upward');
    map.byDirection = true;
    flushSync();
    expect(q('[data-testid="legend"]')?.textContent).toContain('skips a layer');
  });

  it('turning off Tests shrinks every block’s file count (Show, at L0)', async () => {
    const map = render(port());
    await settle();
    expect(q('[data-testid="block-note-p/a"]')?.textContent?.trim()).toBe('3 files');
    map.toggleRole('test');
    flushSync();
    expect(q('[data-testid="block-note-p/a"]')?.textContent?.trim()).toBe('2 files');
  });

  it('finds a module from a picker of every module, grouped as the map is', async () => {
    render(port());
    await settle();
    expect(q('[data-testid="codemap-find"] [data-select-trigger], [data-testid="codemap-find"] button')).not.toBeNull();
  });

  it('the referenced libraries are one block: selecting it lists the most used', async () => {
    render(port());
    await settle();
    (q('[data-testid="codemap-libraries"]') as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="inspector-title"]')?.textContent).toBe('14 packages');
    expect(q('[data-testid="codemap-inspector"]')?.textContent).toContain('serde');
  });

  it('the zoom overlay on the diagram scales it, and % goes back to fit', async () => {
    const map = render(port());
    await settle();
    const width = () => Number(q('[data-testid="codemap-modules"] svg')?.getAttribute('width'));
    const fit = width();
    (q('[data-graph-zoom="in"]') as HTMLButtonElement).click();
    flushSync();
    expect([map.zoom, width()]).toEqual([1.25, Math.round(fit * 1.25)]);
    (q('[data-graph-zoom="reset"]') as HTMLButtonElement).click();
    flushSync();
    expect([map.zoom, width()]).toEqual([1, fit]);
  });

  it('a host with its own header takes the controls and the side panel out', async () => {
    render(port(), { controls: false, side: false });
    await settle();
    expect([q('[data-testid="codemap-search"]'), q('[data-testid="codemap-group"]'), q('[data-testid="codemap-side"]')]).toEqual([null, null, null]);
    expect(q('[data-testid="legend"]')).not.toBeNull();
  });

  it('the diagram and the side panel scroll on their own', async () => {
    render(port());
    await settle();
    for (const id of ['codemap-canvas', 'codemap-side']) {
      const pane = q(`[data-testid="${id}"]`) as HTMLElement;
      expect([pane.className.includes('overflow-auto'), pane.getAttribute('tabindex')], id).toEqual([true, '0']);
    }
  });

  it('says why the map could not be read', async () => {
    render({ ...port(), level0: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Service Unavailable' })) });
    await settle();
    expect(q('[data-testid="codemap-error"]')?.textContent).toContain('Service Unavailable');
  });

  it('under a focus, Hidden coupling and Complexity draw only what the focus is in', async () => {
    const p = port();
    const focusedL0 = { ...L0, focus: { kind: 'module' as const, id: 'p/a', members: ['p/a'], neighbours: ['p/b'] } };
    p.level0 = vi.fn((_g, f) => ok(f ? focusedL0 : L0));
    p.cochange = vi.fn(() => ok({ hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }, { a: 'p/b', b: 'p/c', together: 6, strength: 0.6 }], pairs: 3, commits: 40, minTogether: 5 }));
    p.complexity = vi.fn(() => ok({
      total: { functions: 15, complex: 4, maxCyclo: 24, files: 2 }, maintainability: null,
      modules: [
        { id: 'p/a', label: 'p/a', kind: 'module' as const, functions: 10, complex: 3, maxCyclo: 24, files: 1, children: [] },
        { id: 'p/c', label: 'p/c', kind: 'module' as const, functions: 5, complex: 1, maxCyclo: 12, files: 1, children: [] },
      ],
    }));
    const map = render(p);
    await settle();
    await map.setFocus(p, { kind: 'module', id: 'p/a' });
    void map.showLens(p, 'coupling');
    await settle();
    expect([...target.querySelectorAll('[data-pair]')].map((e) => e.getAttribute('data-pair'))).toEqual(['p/a|p/c']);
    void map.showLens(p, 'complexity');
    await settle();
    expect(q('[data-testid="complexity-overall"]')?.textContent).toContain('3 of 10 functions');
  });

  it('a focused module’s panel: its numbers as stat cards, Open files, and Clear focus', async () => {
    const p = port();
    const map = render(p);
    await settle();
    await map.setFocus(p, { kind: 'module', id: 'p/a' });
    await settle();
    const symbols = q('[data-testid="inspector-stat-Symbols"]') as HTMLElement;
    expect(symbols.matches('[data-stat-card]')).toBe(true);
    expect(q('[data-testid="codemap-open"]')?.textContent?.trim()).toBe('Open files');
    (q('[data-testid="codemap-clear-focus"]') as HTMLButtonElement).click();
    await settle();
    expect([map.focus, (p.level0 as ReturnType<typeof vi.fn>).mock.lastCall]).toEqual([null, ['layers', null]]);
    expect(q('[data-testid="codemap-clear-focus"]')).toBeNull();
  });

  it('a focused segment’s panel describes the group, with Clear focus and no Open files', async () => {
    const p = port();
    p.level0 = vi.fn((_g, f) => ok(f ? { ...L0, focus: { kind: 'segment' as const, id: 'L0', members: ['p/a'], neighbours: ['p/b'] } } : L0));
    const map = render(p);
    await settle();
    await map.setFocus(p, { kind: 'segment', id: 'L0' });
    await settle();
    expect(q('[data-testid="inspector-title"]')?.textContent).toBe('Layer L0');
    expect([q('[data-testid="codemap-clear-focus"]') !== null, q('[data-testid="codemap-open"]')]).toEqual([true, null]);
    expect(q('[data-testid="codemap-takeaways"]')).not.toBeNull();
  });

  // Review r3 N4(b): the page passes a lens's own failure to the lens.
  it('a lens whose own read failed shows why with Retry, and Retry reads it again', async () => {
    const p = port();
    p.cochange = vi.fn()
      .mockReturnValueOnce(Promise.resolve({ ok: false as const, error: 'Gateway Timeout' }))
      .mockReturnValue(ok({ hidden: [], pairs: 0, commits: 3, minTogether: 5 }));
    const map = render(p);
    await settle();
    void map.showLens(p, 'coupling');
    await settle();
    const failed = q('[data-testid="codemap-coupling"] [data-testid="lens-failed"]') as HTMLElement;
    expect(failed.textContent).toContain('Gateway Timeout');
    ([...failed.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Retry') as HTMLButtonElement).click();
    await settle();
    expect(p.cochange).toHaveBeenCalledTimes(2);
    expect(q('[data-testid="lens-failed"]')).toBeNull();
  });

  // Review r3 N4(a): on a narrow screen the list stands in for the lens, and says the same.
  it('on a narrow screen a lens whose read failed shows why with Retry, not an empty list', async () => {
    const width = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth');
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 400 });
    vi.stubGlobal('ResizeObserver', class {
      #cb: ResizeObserverCallback;
      constructor(cb: ResizeObserverCallback) { this.#cb = cb; }
      observe(el: Element) { this.#cb([{ target: el, contentRect: { width: 400 } } as unknown as ResizeObserverEntry], this as unknown as ResizeObserver); }
      unobserve() {}
      disconnect() {}
    });
    try {
      const p = port();
      p.cochange = vi.fn(() => Promise.resolve({ ok: false as const, error: 'Gateway Timeout' }));
      const map = render(p);
      await settle();
      void map.showLens(p, 'coupling');
      await settle();
      expect(q('[data-testid="codemap-list"]')?.textContent).toContain('Gateway Timeout');
      expect([...target.querySelectorAll('[data-testid="codemap-list"] button')].some((b) => b.textContent?.trim() === 'Retry')).toBe(true);
    } finally {
      vi.unstubAllGlobals();
      if (width) Object.defineProperty(HTMLElement.prototype, 'clientWidth', width);
      else delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth;
    }
  });
});
