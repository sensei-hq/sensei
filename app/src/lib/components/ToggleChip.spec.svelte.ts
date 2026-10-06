// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import ToggleChipHarness from './ToggleChip.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function chip(props: Record<string, unknown>) {
  const m = mountComponent(ToggleChipHarness, { testid: 'chip', ...props } as never);
  cleanup.push(m.destroy);
  return { m, el: m.container.querySelector('[data-testid="chip"]') as HTMLButtonElement };
}

describe('ToggleChip', () => {
  // THE DEFECT THIS COMPONENT EXISTS TO FIX, asserted as a property rather than
  // as a colour. Five screens carried a static `bg-transparent` beside a
  // conditional `bg-primary`; UnoCSS emitted both and `bg-transparent` won by
  // source order, so a pressed chip computed to `rgba(0, 0, 0, 0)` behind
  // `oklch(0.975 0.008 85)` text — near-white on paper, unreadable. Measured in
  // the browser on the shipped Structure screen, where "file" and "calls" were
  // invisible while pressed.
  //
  // Mutation that must break this test: add `bg-transparent` back to the static
  // class list.
  it('never carries both backgrounds at once', () => {
    const { el } = chip({ label: 'module', pressed: true });
    expect(el.className).toMatch(/\bbg-primary\b/);
    expect(el.className).not.toMatch(/\bbg-transparent\b/);
  });

  it('an unpressed chip is transparent and inked, not primary', () => {
    const { el } = chip({ label: 'file', pressed: false });
    expect(el.className).toMatch(/\bbg-transparent\b/);
    expect(el.className).not.toMatch(/\bbg-primary\b/);
    expect(el.className).not.toMatch(/\btext-on-primary\b/);
  });

  // A toggle, not navigation — so the state assistive technology hears is
  // `pressed`. `aria-current` belongs on links that go somewhere.
  it('states its pressed state to assistive technology', () => {
    expect(chip({ label: 'calls', pressed: true }).el.getAttribute('aria-pressed')).toBe('true');
    expect(chip({ label: 'calls', pressed: false }).el.getAttribute('aria-pressed')).toBe('false');
  });

  it('calls back on click', () => {
    const { m, el } = chip({ label: 'calls', pressed: false });
    el.click();
    flushSync();
    expect(m.container.querySelector('[data-testid="presses"]')?.textContent).toBe('1');
  });

  // Two sizes, because the diagram bars carry a primary axis and a secondary
  // one, and a single size made the kind row compete with the level row.
  it('sizes down for a secondary axis', () => {
    const { el } = chip({ label: 'imports', pressed: false, size: 'xs' });
    expect(el.className).toMatch(/\btext-xs\b/);
    expect(el.className).not.toMatch(/\btext-sm\b/);
  });
});
