// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import ScreenHarness from './Screen.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const part = (root: HTMLElement, name: string) =>
  root.querySelector(`[data-component="${name}"]`) as HTMLElement | null;

describe('Screen — one fixed header over one scroll body (#293)', () => {
  it('fills its container as a column that never scrolls itself', () => {
    const m = mountComponent(ScreenHarness, {});
    cleanup.push(m.destroy);
    const screen = part(m.container, 'screen')!;
    expect(screen.className).toMatch(/\bh-full\b/);
    expect(screen.className).toMatch(/\bflex-col\b/);
    expect(screen.className).toMatch(/\boverflow-hidden\b/);
  });

  it('keeps the header out of the scroll body, so it stays in place', () => {
    const m = mountComponent(ScreenHarness, {});
    cleanup.push(m.destroy);
    const header = part(m.container, 'screen-header')!;
    const body = part(m.container, 'screen-body')!;
    expect(header.textContent).toContain('Title');
    expect(header.className).toMatch(/\bshrink-0\b/);
    expect(body.contains(header)).toBe(false);
  });

  it('makes the body the one scroll owner', () => {
    const m = mountComponent(ScreenHarness, {});
    cleanup.push(m.destroy);
    const body = part(m.container, 'screen-body')!;
    expect(body.textContent).toContain('Body');
    expect(body.className).toMatch(/\bflex-1\b/);
    expect(body.className).toMatch(/\bmin-h-0\b/);
    expect(body.className).toMatch(/\boverflow-y-auto\b/);
  });

  it('renders no header slot when the screen has none', () => {
    const m = mountComponent(ScreenHarness, { withHeader: false });
    cleanup.push(m.destroy);
    expect(part(m.container, 'screen-header')).toBeNull();
    expect(part(m.container, 'screen-body')!.textContent).toContain('Body');
  });

  it('in fill mode, hands the body to panes that scroll themselves', () => {
    const m = mountComponent(ScreenHarness, { fill: true });
    cleanup.push(m.destroy);
    const body = part(m.container, 'screen-body')!;
    expect(body.dataset.fill).toBe('true');
    expect(body.className).toMatch(/\bflex-1\b/);
    expect(body.className).toMatch(/\bmin-h-0\b/);
    expect(body.className).toMatch(/\bflex-col\b/);
    expect(body.className).toMatch(/\boverflow-hidden\b/);
    expect(body.className).not.toMatch(/\boverflow-y-auto\b/);
  });

  it('carries a screen\'s own attributes on its root, so tests can find it', () => {
    const m = mountComponent(ScreenHarness, {});
    cleanup.push(m.destroy);
    const screen = part(m.container, 'screen')!;
    expect(screen.dataset.screen).toBe('probe');
    expect(screen.dataset.extra).toBe('x');
    expect(screen.dataset.component).toBe('screen');
  });
});
