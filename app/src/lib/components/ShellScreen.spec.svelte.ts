// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
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

/** A host store over a reactive map, as appState's config is. */
function makeStore(initial: Record<string, PanelState> = {}, fail = false) {
  const views = $state<Record<string, PanelState>>(initial);
  const set = vi.fn(async (id: string, next: PanelState) => {
    if (fail) throw new Error('Daemon config write failed (500): boom');
    views[id] = next;
  });
  return { views, set, store: { get: (id: string) => views[id], set } };
}

// The `@sensei/ui` wrapper over rokkit's ScreenShell (D87, contract §8): it
// connects the glyph mode to the marks and the view id to the host's panel store.
describe.skip('pending: shell-store — ShellScreen', () => {
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

  it('carries the view id onto the shell, so a test or a host can find the view', () => {
    const m = mountComponent(Harness, { viewId: 'workspace.diagrams.er' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-shell]')!.getAttribute('data-view')).toBe('workspace.diagrams.er');
  });
});
