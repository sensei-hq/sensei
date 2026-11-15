// @vitest-environment jsdom
// Diagrams on the ScreenShell (D87, contract §4/§7/§8/§9). The migration rule is
// "layout only": the two pages keep their bodies, controls, inspector, budget and
// focus behaviour, and the header's four toggles become the shell's band toggles.
//
// The two page specs (`[lens]/page.spec.svelte.ts` unchanged, `schema/page.spec.svelte.ts`
// with the one sanctioned structural assertion) stay the acceptance bar; this file
// pins what the shell itself brings: the view id, the bands from the view's data, the
// inspector in the sidebar slot, and the store behind the toggles.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import LensPage from './[lens]/+page.svelte';
import SchemaPage from './schema/+page.svelte';
import { CODEMAP_HOST, CodeMapHost } from './codemap-host.svelte.js';
import { DIAGRAM_VIEWS } from './diagrams-nav.js';
import { KANJI_ICONS } from '$lib/glyph-vocabulary';
import type { CodeMapPort, Level0, Level1, Lens } from '@sensei/codemap';

const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
  blocks: [{
    id: 'p/a', label: 'p/a', group: 'L0', package: 'p', files: 2, symbols: 9,
    roles: { code: 2, test: 0, docs: 0, config: 0, generated: 0 }, hub: false, weight_in: 0, weight_out: 0, folded: [],
  }],
  bands: [], budget: { drawn: 1, modules: 1, bands: 0, edges: 0 }, libraries: { count: 0, top: [] },
};
const L1: Level1 = { level: 'file', module: 'p/a', dependsOn: [], usedBy: [], columns: [] };
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const port = (): CodeMapPort => ({
  level0: vi.fn(() => ok(L0)), module: vi.fn(() => ok(L1)), file: vi.fn(),
  zones: vi.fn(() => ok({ points: [], meanDistance: 0, coverage: { points: 0, unplaceable: 0 } })),
  cochange: vi.fn(() => ok({ hidden: [], pairs: 0, commits: 0, minTogether: 5 })),
  // One row above iterates every lens, Complexity among them (#1).
  complexity: vi.fn(() => ok({
    total: { functions: 0, complex: 0, maxCyclo: 0, files: 0, tier: 'none' },
    maintainability: { score: 100, tier: 'none' },
    modules: [],
  })),
} as unknown as CodeMapPort);

// The two host-wide contracts a screen reads from context (packages/ui): the store
// the bands are kept in, and the glyph mode. `Symbol.for`, so the same key the
// provider uses.
const VIEW_STATES = Symbol.for('sensei.view-states');
const GLYPHS = Symbol.for('sensei.glyph-mode');

interface Store {
  get: (id: string) => unknown;
  set: (id: string, next: unknown) => Promise<void>;
  error: () => string | null;
  stored: Record<string, unknown>;
}
const storeOf = (stored: Record<string, unknown> = {}): Store => ({
  stored,
  get: vi.fn((id: string) => stored[id]),
  set: vi.fn(async (id: string, next: unknown) => void (stored[id] = next)),
  error: () => null,
});

const settle = async () => { for (let i = 0; i < 4; i++) await new Promise((r) => setTimeout(r, 0)); flushSync(); };
let app: Record<string, unknown> | null = null;
let target: HTMLElement;
const drop = () => { if (app) unmount(app); app = null; target?.remove(); };
afterEach(() => { drop(); vi.unstubAllGlobals(); });

const q = (sel: string) => target.querySelector(sel) as HTMLElement | null;
const all = (sel: string) => [...target.querySelectorAll(sel)] as HTMLElement[];
const toggle = (name: string) => q(`[data-toolbar-item][aria-label="${name}"]`) as HTMLButtonElement;

function mountPage(Page: unknown, props: unknown, host: object, opts: { glyphs?: 'kanji' | 'icon'; store?: Store } = {}) {
  target = document.body.appendChild(document.createElement('div'));
  const store = opts.store ?? storeOf();
  app = mount(Page as never, {
    target,
    props: { data: props } as never,
    context: new Map<unknown, unknown>([
      [CODEMAP_HOST, host],
      [VIEW_STATES, store],
      [GLYPHS, () => opts.glyphs ?? 'kanji'],
    ]),
  });
  return { store, host };
}

function renderLens(lens: Lens, opts: { glyphs?: 'kanji' | 'icon'; store?: Store } = {}) {
  const data = $state({
    workspaceId: 'w', lens, view: DIAGRAM_VIEWS.find((v) => v.id === lens)!,
    workspace: { id: 'w', name: 'sensei', repos_count: 3 },
  });
  const host = new CodeMapHost(() => 'w', port());
  return { ...mountPage(LensPage, data, host, opts), data, map: host };
}

// A design, however small: every assertion in #15 is about a page that drew one
// (an empty payload is the `schema-none` state, which is #15's sibling in the
// page's own spec).
const PAYLOAD = {
  version: 3,
  tables: [{ schema: 'app', name: 'customers', kind: 'table', columns: [{ name: 'id', type: 'uuid', pk: true, nn: true }] }],
  refs: [], entities: [], deps: [], enums: [],
};
function renderSchema(opts: { store?: Store } = {}) {
  vi.stubGlobal('fetch', vi.fn(async () =>
    ({ ok: true, status: 200, statusText: 'OK', json: async () => PAYLOAD }) as Response));
  const data = { workspaceId: 'w', workspace: { id: 'w', name: 'sensei', repos_count: 3 } };
  return mountPage(SchemaPage, data, new CodeMapHost(() => 'w', port()), opts);
}

describe('diagrams-shell — the diagrams render on the ScreenShell', () => {
  it('one id per kind of view: workspace.diagrams.<lens>, and Database · ER as workspace.diagrams.er', async () => {
    for (const view of DIAGRAM_VIEWS.filter((v) => v.lens)) {
      drop();
      renderLens(view.lens!);
      await settle();
      expect(q('[data-screen-shell]')?.dataset.view, view.id).toBe(`workspace.diagrams.${view.id}`);
    }
    drop();
    renderSchema();
    await settle();
    expect(q('[data-screen-shell]')?.dataset.view).toBe('workspace.diagrams.er');
  });

  it('the lens page is one shell, titled and described by the view it draws', async () => {
    renderLens('structure');
    await settle();
    expect(q('[data-screen-title]')?.textContent?.trim()).toBe('Structure');
    expect(q('[data-screen-description]')?.textContent?.trim()).toBe(DIAGRAM_VIEWS[0].shows);
    // The whole level tag, as the old header drew it: `L<depth> · <level>`.
    expect(q('[data-screen-context]')?.textContent?.trim()).toBe('L0 · modules');
    expect(q('[data-screen-mark]')?.textContent).toBe('構');
  });

  it('the mark follows the glyph mode: the view’s icon when icons are chosen', async () => {
    renderLens('wheel', { glyphs: 'icon' });
    await settle();
    const mark = q('[data-screen-mark]');
    expect(mark?.className).toBe(DIAGRAM_VIEWS[1].icon);
    expect(mark?.textContent).not.toContain('輪');
  });

  it('the info band is the view’s three columns, the last one the next step', async () => {
    renderLens('structure');
    await settle();
    // `?? ''`, so a missing band fails as a missing band rather than as a
    // `toContain` on `undefined` (readable message, same behaviour).
    const info = q('[data-screen-info]')?.textContent ?? '';
    expect(info).toContain('Use it to');
    expect(info).toContain(DIAGRAM_VIEWS[0].use);
    expect(info).toContain('How to read it');
    expect(info).toContain(DIAGRAM_VIEWS[0].read);
    expect(info).toContain('What to do');
    expect(info).toContain(DIAGRAM_VIEWS[0].todo);
  });

  it('the stats band is the workspace’s numbers, with the repos beside the modules', async () => {
    renderLens('structure');
    await settle();
    const labels = all('[data-screen-stats] [data-stat-card] [data-stat-card-label]').map((n) => n.textContent?.trim());
    expect(labels.slice(0, 3)).toEqual(['Modules', 'Repos', 'Symbols']);
    expect(all('[data-screen-stats] [data-stat-card]')[1]?.querySelector('[data-stat-card-value]')?.textContent).toBe('3');
  });

  it('the controls are the filter band, inside the element the old header used', async () => {
    renderLens('structure');
    await settle();
    const band = q('[data-screen-filters] [data-testid="diagram-controls"]');
    expect(band).not.toBeNull();
    expect(band?.querySelector('[data-testid="codemap-controls"]')).not.toBeNull();
  });

  it('the inspector is the shell’s sidebar, and the side band closes it', async () => {
    const { store } = renderLens('structure');
    await settle();
    expect(q('[data-screen-sidebar] [data-testid="codemap-side"]')).not.toBeNull();
    toggle('Side panel').click();
    await settle();
    expect(q('[data-screen-sidebar]')).toBeNull();
    expect(store.set).toHaveBeenCalledWith('workspace.diagrams.structure', expect.objectContaining({ side: false }));
  });

  it('the budget line is the footer', async () => {
    renderLens('structure');
    await settle();
    expect(q('[data-screen-footer] [data-testid="diagram-budget"]')?.textContent).toBeTruthy();
  });

  it('the focus chip sits at the top of the body, outside the title bar, and its ✕ clears', async () => {
    const { map } = renderLens('structure');
    await settle();
    await map.state.setFocus(map.port, { kind: 'module', id: 'p/a' });
    await settle();
    const chip = q('[data-screen-body] [data-testid="diagram-focus"]');
    expect(chip).not.toBeNull();
    expect(q('[data-screen-header] [data-testid="diagram-focus"]')).toBeNull();
    expect(chip?.textContent).toContain('p/a');
    (chip?.querySelector('button[aria-label="Clear focus"]') as HTMLButtonElement).click();
    await settle();
    expect(q('[data-testid="diagram-focus"]')).toBeNull();
  });

  it('the body is still the map, with the lens on it', async () => {
    renderLens('wheel');
    await settle();
    expect(q('[data-screen-body] [data-testid="codemap-screen"]')?.getAttribute('data-lens')).toBe('wheel');
  });

  it('the four band toggles are the old header’s, with their names and pressed state', async () => {
    renderLens('structure');
    await settle();
    expect(all('[data-screen-tools] [data-toolbar-item]').map((b) => b.getAttribute('aria-label')))
      .toEqual(["What it's for", 'Workspace numbers', 'Controls', 'Side panel']);
    expect(toggle("What it's for").getAttribute('aria-pressed')).toBe('true');
  });

  it('a first visit opens everything; a toggle keeps the whole state under this view id', async () => {
    const { store } = renderLens('wheel');
    await settle();
    expect([q('[data-screen-info]'), q('[data-screen-stats]'), q('[data-screen-filters]'), q('[data-screen-sidebar]')].every(Boolean)).toBe(true);
    toggle('Workspace numbers').click();
    await settle();
    expect(store.set).toHaveBeenCalledWith('workspace.diagrams.wheel', { info: true, stats: false, filters: true, side: true });
  });

  it('the next visit reopens as it was left, per view id', async () => {
    renderLens('structure', { store: storeOf({ 'workspace.diagrams.structure': { info: false } }) });
    await settle();
    expect(q('[data-screen-info]')).toBeNull();
    expect(q('[data-screen-stats]')).not.toBeNull();
  });

  it('two lenses keep their own bands', async () => {
    renderLens('wheel', { store: storeOf({ 'workspace.diagrams.wheel': { side: false } }) });
    await settle();
    expect(q('[data-screen-sidebar]')).toBeNull();
    drop();
    renderLens('zones', { store: storeOf({ 'workspace.diagrams.wheel': { side: false } }) });
    await settle();
    expect(q('[data-screen-sidebar]')).not.toBeNull();
  });

  it('Database · ER is workspace.diagrams.er, its sidebar in the slot, its tab body unchanged', async () => {
    renderSchema();
    await settle();
    expect(q('[data-screen-sidebar] [data-testid="schema-sidebar"] [data-graph-design-sidebar]')).not.toBeNull();
    expect(q('[data-screen-body] [data-testid="schema-layout"]')).not.toBeNull();
    expect(q('[data-screen-body] [data-testid="schema-tabs"]')).not.toBeNull();
    expect(q('[data-screen-footer] [data-testid="diagram-budget"]')?.textContent).toBe('schema model v3');
  });

  // The canvas is the app's own (user, 2026-11-06): tables and views on one picture,
  // and a press walks into a neighbourhood instead of opening a page.
  it('the ER page’s Diagram tab is the app canvas, and a press walks into a neighbourhood', async () => {
    renderSchema();
    await settle();
    const tab = [...document.querySelectorAll('[role="tab"]')].find((t) => /diagram/i.test(t.textContent ?? ''));
    expect(tab).toBeTruthy();
    (tab as HTMLElement).click();
    await settle();
    expect(q('[data-testid="schema-diagram"] [data-testid="er-canvas"]')).not.toBeNull();
    const card = q('[data-testid="er-canvas"] [data-graph-node="app.customers"]');
    expect(card).not.toBeNull();
    (card as HTMLElement).click();
    await settle();
    expect(q('[data-testid="er-neighbourhood"]')?.getAttribute('data-centre')).toBe('app.customers');
    (q('[data-testid="er-design-back"]') as HTMLButtonElement).click();
    await settle();
    expect(q('[data-testid="er-neighbourhood"]')).toBeNull();
    expect(q('[data-testid="er-canvas"] [data-graph-node="app.customers"]')).not.toBeNull();
  });

  // The Details tab's six-column field table widened the page instead of scrolling
  // inside it (user, 2026-11-06). The rule is a `<style>` block, so this is the pin
  // that keeps it — no mounted-DOM assertion can see a stylesheet in jsdom.
  it('the Details tab’s field table scrolls inside its column', () => {
    const page = readFileSync(
      join(process.cwd(), 'src/routes/(workspace)/workspace/[id]/diagrams/schema/+page.svelte'),
      'utf8',
    );
    expect(page).toMatch(/\[data-details-fit\] :global\(\[data-graph-design-scroll\]\)\s*\{[^}]*overflow-x: auto/);
    expect(page).toMatch(/data-details-fit/);
  });

  // The inspect specs keep their selectors: rokkit 1.15.2 hands the hooks back
  // (per-band toggle attrs, band attrs, stat attrs), so the pages carry the old
  // `data-testid`s instead of every spec learning a role and an accessible name.
  it('the inspect specs’ hooks come from the shell’s passthrough, untouched by the migration', async () => {
    renderLens('structure');
    await settle();
    const lens = [
      'diagram-about-toggle', 'diagram-stats-toggle', 'diagram-controls-toggle', 'diagram-side-toggle',
      'diagram-stats', 'diagram-budget', 'diagram-stat-Modules', 'diagram-stat-Cycles',
      'diagram-controls', 'codemap-screen', 'codemap-side',
    ];
    expect(lens.filter((h) => !q(`[data-testid="${h}"]`))).toEqual([]);
    // The toggle the spec clicks is the one that says whether its band is open.
    expect(['diagram-about-toggle', 'diagram-stats-toggle', 'diagram-controls-toggle', 'diagram-side-toggle']
      .map((h) => q(`[data-testid="${h}"]`)?.getAttribute('aria-pressed'))).toEqual(['true', 'true', 'true', 'true']);

    drop();
    renderSchema();
    await settle();
    expect(['diagram-stats-toggle', 'diagram-side-toggle', 'diagram-stats', 'diagram-budget',
      'schema-screen', 'schema-layout', 'schema-sidebar', 'schema-tabs']
      .filter((h) => !q(`[data-testid="${h}"]`))).toEqual([]);
  });

  // Review r1 F1: the ER page has its own `cardOf`, so the lens pins above say
  // nothing about it — and `schema-inspect` reads this card's own hooks.
  it('the ER stat cards keep the hooks schema-inspect reads off them', async () => {
    renderSchema();
    await settle();
    // The label counts, as the design does: one table is `Table`.
    expect(q('[data-testid="diagram-stat-Table"]')).not.toBeNull();
    expect(q('[data-testid="diagram-stat-Table"] [data-stat-card-value]')?.textContent).toBe('1');
  });

  // Review r1 F3 (N5a's ruling): rokkit draws a card's icon only in icon mode and
  // its `mark` in kanji mode, so a card without a mark shows nothing by default —
  // where the old header always showed the stat's Solar icon.
  it('every stat card carries its own mark and tone, and its icon in icon mode', async () => {
    renderLens('structure');
    await settle();
    expect(q('[data-testid="diagram-stat-Modules"] [data-stat-card-mark]')?.textContent).toBe('構');
    expect(q('[data-testid="diagram-stat-Repos"] [data-stat-card-mark]')?.textContent).toBe('場');
    expect(q('[data-testid="diagram-stat-Cycles"] [data-stat-card-mark]')?.textContent).toBe('環');
    // The tone the old spec pinned on this card, and still the shell's hook (#133).
    expect(q('[data-testid="diagram-stat-Cycles"]')?.getAttribute('data-tone')).toBe('warning');
    // #133: every mark is the vocabulary's, so icon mode can draw it as its icon.
    const marks = all('[data-screen-stats] [data-stat-card-mark]').map((n) => n.textContent ?? '');
    expect(marks.every((m) => KANJI_ICONS[m])).toBe(true);

    drop();
    renderLens('structure', { glyphs: 'icon' });
    await settle();
    expect(q('[data-testid="diagram-stat-Modules"] [data-stat-card-icon]')?.className)
      .toBe('i-solar-box-minimalistic-bold-duotone');

    drop();
    renderSchema();
    await settle();
    expect(q('[data-testid="diagram-stat-Table"] [data-stat-card-mark]')?.textContent).toBe('表');
  });

  // Review r1 F4 (N5b's ruling): the shell's sidebar slot has no narrow rule, and
  // the old map drew its panel only `{#if side && !narrow}`. The page measures its
  // own area — the width the map used to — so hiding the panel cannot re-widen it.
  it('a narrow window keeps the sidebar closed, and a wide one shows it', async () => {
    const width = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth');
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 400 });
    vi.stubGlobal('ResizeObserver', class {
      constructor(cb: ResizeObserverCallback) {
        void cb;
      }
      observe(el: Element) {
        void el;
      }
      unobserve() {}
      disconnect() {}
    });
    try {
      renderLens('structure');
      await settle();
      expect(q('[data-screen-body]')).not.toBeNull();
      expect(q('[data-screen-sidebar]')).toBeNull();
    } finally {
      if (width) Object.defineProperty(HTMLElement.prototype, 'clientWidth', width);
      else delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth;
    }
  });

  it('the legacy diagram keys migrate once, into every diagram’s view id — after the views are read', () => {
    const layout = readFileSync(join(process.cwd(), 'src/routes/+layout.svelte'), 'utf8');
    expect(layout).toMatch(/DIAGRAM_VIEW_IDS/);
    // One call, and it is reactive. `migrateLegacyViews` throws before the
    // daemon's map has been read ("refusing to migrate them"), and the map is
    // read by the layouts' `appState.load()`, which a layout's top level runs
    // before: a synchronous call passes a bare `/migrateLegacyViews\(/` and then
    // throws on start in any browser still holding the old keys. No spec mounts
    // the root layout, so this source check is that guard.
    expect(layout.match(/migrateLegacyViews\(/g) ?? []).toHaveLength(1);
    const effect = layout.match(/\$effect\(\s*\(\)\s*=>\s*\{[\s\S]*?migrateLegacyViews\(DIAGRAM_VIEW_IDS\)[\s\S]*?\}\);/)?.[0] ?? '';
    expect(effect).not.toBe('');
    // …and gated on a read that worked: `loaded` is also true when the read
    // failed, and that leaves the map unread.
    expect(effect).toContain('appState.loaded');
    expect(effect).toContain('configError');
  });
});
