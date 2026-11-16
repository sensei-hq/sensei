// @vitest-environment jsdom
import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest';
import { flushSync, tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { PanelState } from '@rokkit/app';
import Harness from './ShellScreen.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// rokkit's band toggles, by the label each carries (messages.screen).
const LABELS = { info: 'What it’s for', stats: 'Numbers', filters: 'Filters', side: 'Side panel' } as const;

function toggle(container: HTMLElement, band: keyof typeof LABELS): HTMLButtonElement {
  const label = LABELS[band];
  const button = [...container.querySelectorAll<HTMLButtonElement>('[data-screen-tools] button')].find(
    (b) => b.getAttribute('aria-label') === label || b.title === label || b.textContent?.trim() === label,
  );
  expect(button, `the ${band} toggle`).toBeTruthy();
  return button!;
}

const bands = (c: HTMLElement) => ({
  info: Boolean(c.querySelector('[data-screen-info-label]')),
  stats: Boolean(c.querySelector('[data-screen-stats]')),
  filters: Boolean(c.querySelector('[data-screen-filters]')),
  side: Boolean(c.querySelector('[data-screen-sidebar]')),
});

// r2 F2: a working Map-backed localStorage (jsdom's would hide a guarded write),
// so "the wrapper persists through the store, never localStorage" is checkable.
const storage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (k: string) => storage.get(k) ?? null,
  setItem: (k: string, v: string) => void storage.set(k, v),
  removeItem: (k: string) => void storage.delete(k),
  clear: () => storage.clear(),
  key: (i: number) => [...storage.keys()][i] ?? null,
  get length() { return storage.size; },
});
beforeEach(() => storage.clear());

/** A host store over a reactive map, as appState's config is. `ctl.fail` refuses
 *  saves by rejecting ONLY — it does not also report the failure through
 *  `error()`, so the wrapper must handle the rejection itself (a wrapper that
 *  swallows it, or never catches it, fails). `ctl.error` is the store's read
 *  error (appState's `viewsError` for an unreadable `ui.views`). */
function makeStore(initial: Record<string, PanelState> = {}, fail = false, error: string | null = null) {
  const views = $state<Record<string, PanelState>>(initial);
  const ctl = $state({ fail, error });
  const set = vi.fn(async (id: string, next: PanelState) => {
    if (ctl.fail) throw new Error('Daemon config write failed (500): boom');
    views[id] = next;
  });
  return { views, set, ctl, store: { get: (id: string) => views[id], set, error: () => ctl.error } };
}

const notice = (c: HTMLElement) => c.querySelector('[data-screen-body] [data-component="notice"]');

// The `@sensei/ui` wrapper over rokkit's ScreenShell (D87, contract §8): it
// connects the glyph mode to the marks and the view id to the host's panel store.
describe('ShellScreen', () => {
  // #420: rokkit's shell is `height: 100%` with the body as the only scroller.
  // Against an auto-height parent it grows and the window clips it — the Relay
  // screens wrapped it in a bare div and lost their scroll. The wrapper fills
  // its container itself, so no screen can get this wrong again.
  it('fills its container: the shell root carries h-full and min-h-0', () => {
    const { container, destroy } = mountComponent(Harness, { viewId: 'test.view', glyphs: 'kanji' });
    cleanup.push(destroy);
    const root = container.querySelector('[data-screen-shell]');
    expect(root?.classList.contains('h-full'), 'h-full').toBe(true);
    expect(root?.classList.contains('min-h-0'), 'min-h-0').toBe(true);
  });

  // The shell's sidebar scrolls on its own once the shell fills its container. A
  // scrolling region with nothing focusable in it is out of a keyboard's reach
  // (axe scrollable-region-focusable, serious — the diagrams sidebar, #420 e2e).
  it('the sidebar is keyboard-reachable: it takes focus, so it can be scrolled from the keys', () => {
    const { container, destroy } = mountComponent(Harness, { viewId: 'test.view', glyphs: 'kanji' });
    cleanup.push(destroy);
    const side = container.querySelector('[data-screen-sidebar]');
    expect(side, 'the sidebar').not.toBeNull();
    expect(side?.getAttribute('tabindex')).toBe('0');
  });

  it('draws the kanji in kanji mode', () => {
    const m = mountComponent(Harness, { glyphs: 'kanji' });
    cleanup.push(m.destroy);
    const mark = m.container.querySelector('[data-screen-titlebar] [data-screen-mark]')!;
    expect(mark.textContent).toBe('構');
  });

  it('draws the icon in icon mode: sensei\'s "icon" glyph mode reaches the shell', () => {
    const m = mountComponent(Harness, { glyphs: 'icon' });
    cleanup.push(m.destroy);
    const mark = m.container.querySelector('[data-screen-titlebar] [data-screen-mark]')!;
    expect(mark.classList.contains('i-test:structure')).toBe(true);
    expect(mark.textContent).toBe('');
  });

  it('a first visit (no entry for the view) opens every band', () => {
    const { store } = makeStore();
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    expect(bands(m.container)).toEqual({ info: true, stats: true, filters: true, side: true });
  });

  it('opens the bands as the store remembers them for this view id', () => {
    const { store } = makeStore({ 'test.view': { info: false, side: false }, other: { stats: false } });
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    expect(bands(m.container)).toEqual({ info: false, stats: true, filters: true, side: false });
  });

  it('two view ids read two states', () => {
    const { store } = makeStore({ 'test.view': { stats: false }, other: { side: false } });
    const a = mountComponent(Harness, { store, viewId: 'test.view' });
    const b = mountComponent(Harness, { store, viewId: 'other' });
    cleanup.push(a.destroy, b.destroy);
    expect(bands(a.container)).toMatchObject({ stats: false, side: true });
    expect(bands(b.container)).toMatchObject({ stats: true, side: false });
  });

  it('a toggle saves the whole panel state under the view id', async () => {
    const { store, set } = makeStore({ 'test.view': { side: false } });
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    toggle(m.container, 'info').click();
    flushSync();
    expect(set).toHaveBeenCalledTimes(1);
    expect(set.mock.calls[0][0]).toBe('test.view');
    expect(set.mock.calls[0][1]).toMatchObject({ info: false, side: false });
    await tick();
    expect(bands(m.container)).toMatchObject({ info: false, side: false });
  });

  it('a change the store makes (another window\'s save) shows without a remount', () => {
    const { store, views } = makeStore();
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    views['test.view'] = { stats: false };
    flushSync();
    expect(bands(m.container)).toMatchObject({ stats: false, info: true });
  });

  it('a save the store refused leaves the band as stored: the screen never shows a state that was not kept', async () => {
    const { store, set } = makeStore({}, true);
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    toggle(m.container, 'stats').click();
    flushSync();
    expect(set).toHaveBeenCalledTimes(1);
    await Promise.resolve();
    await tick();
    flushSync();
    expect(bands(m.container)).toMatchObject({ stats: true });
  });

  it('with no store provided, the bands open and toggle in the screen alone', () => {
    const m = mountComponent(Harness, {});
    cleanup.push(m.destroy);
    expect(bands(m.container)).toEqual({ info: true, stats: true, filters: true, side: true });
    toggle(m.container, 'side').click();
    flushSync();
    expect(bands(m.container).side).toBe(false);
  });

  // ── r2 F1 (D83): a refused save, or an unreadable store, is shown ─────────

  it('a refused save is shown in the body as a Notice naming the failure; the band stays as stored', async () => {
    const { store } = makeStore({}, true);
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    expect(notice(m.container)).toBeNull();
    toggle(m.container, 'stats').click();
    flushSync();
    await Promise.resolve();
    await tick();
    flushSync();
    expect(notice(m.container)?.textContent).toContain('boom');
    expect(bands(m.container)).toMatchObject({ stats: true });
    expect(m.container.querySelector('[data-testid="body"]'), 'the screen\'s body still renders').toBeTruthy();
  });

  it('the next good save clears the Notice', async () => {
    const { store, ctl } = makeStore({}, true);
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    toggle(m.container, 'stats').click();
    flushSync();
    await Promise.resolve();
    await tick();
    flushSync();
    expect(notice(m.container)).toBeTruthy();
    ctl.fail = false;
    toggle(m.container, 'stats').click();
    flushSync();
    await Promise.resolve();
    await tick();
    flushSync();
    expect(notice(m.container)).toBeNull();
    expect(bands(m.container)).toMatchObject({ stats: false });
  });

  it('a store that could not read its views (an unreadable ui.views) shows the Notice on mount', () => {
    const { store } = makeStore({}, false, 'ui.views could not be read: Unexpected token');
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    expect(notice(m.container)?.textContent).toContain('ui.views could not be read');
    expect(bands(m.container)).toEqual({ info: true, stats: true, filters: true, side: true });
  });

  // ── r2 F2: persistence goes through the store, never localStorage ─────────

  it('with a store, toggles write nothing to localStorage', async () => {
    const { store } = makeStore();
    const m = mountComponent(Harness, { store });
    cleanup.push(m.destroy);
    for (const band of ['info', 'stats', 'filters', 'side'] as const) {
      toggle(m.container, band).click();
      flushSync();
    }
    await tick();
    expect(storage.size).toBe(0);
  });

  it('with no provider, toggles write nothing to localStorage either: the screen alone, no persistence', () => {
    const m = mountComponent(Harness, {});
    cleanup.push(m.destroy);
    toggle(m.container, 'side').click();
    flushSync();
    toggle(m.container, 'info').click();
    flushSync();
    expect(storage.size).toBe(0);
  });

  it('carries the view id onto the shell, so a test or a host can find the view', () => {
    const m = mountComponent(Harness, { viewId: 'workspace.diagrams.er' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-shell]')!.getAttribute('data-view')).toBe('workspace.diagrams.er');
  });
});
