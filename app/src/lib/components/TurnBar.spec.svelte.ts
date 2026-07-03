// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import TurnBarHarness from './TurnBar.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('TurnBar', () => {
  it('renders nothing when turns == 0 (no session material)', () => {
    const m = mountComponent(TurnBarHarness, { turns: 0, corrections: 0 });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="turn-bar"]')).toBeNull();
  });

  it('draws one rect per turn — clean + corrected together', () => {
    const m = mountComponent(TurnBarHarness, { turns: 5, corrections: 2 });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="turn-bar"]');
    expect(svg).toBeTruthy();
    expect(svg!.querySelectorAll('rect').length).toBe(5);
  });

  it('sets data attributes so tests can assert without reading fills', () => {
    const m = mountComponent(TurnBarHarness, { turns: 7, corrections: 3 });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="turn-bar"]') as SVGElement;
    expect(svg.getAttribute('data-turns')).toBe('7');
    expect(svg.getAttribute('data-corrections')).toBe('3');
  });

  it('clamps corrections > turns so a buggy row never overflows the bar', () => {
    const m = mountComponent(TurnBarHarness, { turns: 3, corrections: 10 });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="turn-bar"]');
    expect(svg!.querySelectorAll('rect').length).toBe(3);
  });

  it('exposes an aria label for screen readers', () => {
    const m = mountComponent(TurnBarHarness, { turns: 4, corrections: 1 });
    cleanup.push(m.destroy);
    const svg = m.container.querySelector('[data-component="turn-bar"]') as SVGElement;
    expect(svg.getAttribute('aria-label')).toBe('4 turns, 1 rework');
  });
});
