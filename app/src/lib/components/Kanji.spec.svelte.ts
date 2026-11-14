// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Kanji from './Kanji.svelte';
import KanjiHarness from './Kanji.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Kanji', () => {
  it('renders the glyph string', () => {
    const m = mountComponent(Kanji, { char: '観' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toBe('観');
  });

  it('applies the kanji font-family class', () => {
    const m = mountComponent(Kanji, { char: '聴' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="kanji"]') as HTMLElement;
    expect(el.className).toMatch(/\bkanji\b/);
  });

  it('defaults to size base + accent tone', () => {
    const m = mountComponent(Kanji, { char: '具' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="kanji"]') as HTMLElement;
    expect(el.className).toMatch(/\btext-base\b/);
    expect(el.className).toMatch(/\btext-accent\b/);
  });

  it.each([
    ['xs',   /\btext-xs\b/],
    ['sm',   /\btext-sm\b/],
    ['lg',   /\btext-lg\b/],
    ['xl',   /\btext-xl\b/],
    ['2xl',  /\btext-2xl\b/],
    ['3xl',  /\btext-3xl\b/],
    ['4xl',  /\btext-4xl\b/],
  ] as const)('applies size %s', (size, expected) => {
    const m = mountComponent(Kanji, { char: '探', size });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="kanji"]') as HTMLElement;
    expect(el.className).toMatch(expected);
  });

  it.each([
    ['muted',     /\btext-ink-soft\b/],
    ['success',   /\btext-success\b/],
    ['warning',   /\btext-warning\b/],
    ['watermark', /\btext-accent\b/, /\bopacity-55\b/],
  ] as const)('applies tone %s', (tone, ...expected) => {
    const m = mountComponent(Kanji, { char: '繰', tone });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="kanji"]') as HTMLElement;
    for (const re of expected) expect(el.className).toMatch(re);
  });
});

// #133: the same mark in icon mode — the icon when there is one, at the same
// size and tone; the kanji when there is not, never an empty slot.
describe('Kanji in icon mode', () => {
  it('renders the icon instead of the kanji', () => {
    const m = mountComponent(Kanji, { char: '具', icon: 'i-solar-test-tube-bold-duotone', mode: 'icon', size: 'sm' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="glyph-icon"]') as HTMLElement;
    expect(el.className).toContain('i-solar-test-tube-bold-duotone');
    expect(el.className).toContain('text-sm');
    expect(el.className).toContain('text-accent');
    expect(m.container.querySelector('[data-component="kanji"]')).toBeNull();
  });

  it('keeps the kanji in kanji mode, and when there is no icon', () => {
    for (const props of [
      { char: '具', icon: 'i-solar-test-tube-bold-duotone', mode: 'kanji' as const },
      // A kanji the vocabulary lacks, with no icon given, has nothing to show but itself.
      { char: '鬱', mode: 'icon' as const },
    ]) {
      const m = mountComponent(Kanji, props);
      cleanup.push(m.destroy);
      expect(m.container.querySelector('[data-component="kanji"]')?.textContent).toBe(props.char);
      expect(m.container.querySelector('[data-component="glyph-icon"]')).toBeNull();
    }
  });
});

// #133: with no mode prop, a mark follows the mode the layout provides.
describe('Kanji under a provided glyph mode', () => {
  it('shows the icon when the provided mode is icon', () => {
    const m = mountComponent(KanjiHarness, { char: '名', icon: 'i-solar-user-id-bold-duotone', glyphs: 'icon' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="glyph-icon"]')?.className).toContain('i-solar-user-id-bold-duotone');
  });

  it('shows the kanji when the provided mode is kanji', () => {
    const m = mountComponent(KanjiHarness, { char: '名', icon: 'i-solar-user-id-bold-duotone', glyphs: 'kanji' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="kanji"]')?.textContent).toBe('名');
  });
});

// #133: a mark with no icon of its own takes the vocabulary's; an identity
// mark passes icon={null} and stays kanji.
describe('Kanji and the glyph vocabulary', () => {
  it('takes the icon from the vocabulary when none is given', () => {
    const m = mountComponent(Kanji, { char: '録', mode: 'icon' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="glyph-icon"]')?.className).toContain('i-solar-history-bold-duotone');
  });

  // One rule (#133, 2026-10-27): no mark opts out — a null icon is the vocabulary's.
  it('shows the vocabulary icon for a mark once called an identity', () => {
    const m = mountComponent(Kanji, { char: '書', mode: 'icon', icon: null });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="glyph-icon"]')?.className).toContain('i-solar-book-bold-duotone');
  });

});

// #133: a mark converted from a bare span keeps its own classes — size and tone
// "inherit" add none, and `class` is passed through in both modes.
describe('Kanji with its own classes', () => {
  it('carries the given classes and adds no size or tone of its own', () => {
    const k = mountComponent(Kanji, { char: '探', size: 'inherit', tone: 'inherit', class: 'text-xs text-ink-mute' });
    cleanup.push(k.destroy);
    const el = k.container.querySelector('[data-component="kanji"]') as HTMLElement;
    expect(el.className.split(/\s+/).sort()).toEqual(['kanji', 'text-ink-mute', 'text-xs']);

    const i = mountComponent(Kanji, { char: '探', size: 'inherit', tone: 'inherit', class: 'text-xs text-ink-mute', mode: 'icon' });
    cleanup.push(i.destroy);
    const icon = i.container.querySelector('[data-component="glyph-icon"]') as HTMLElement;
    expect(icon.className.split(/\s+/).sort()).toEqual(['i-solar-magnifer-zoom-in-bold-duotone', 'text-ink-mute', 'text-xs']);
  });
});
