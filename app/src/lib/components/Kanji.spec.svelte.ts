// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Kanji from './Kanji.svelte';

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
    expect(el.className).toMatch(/\btext-primary-z6\b/);
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
    ['muted',     /\btext-ink-z6\b/],
    ['success',   /\btext-success-z6\b/],
    ['warning',   /\btext-warning-z6\b/],
    ['watermark', /\btext-primary-z6\b/, /\bopacity-55\b/],
  ] as const)('applies tone %s', (tone, ...expected) => {
    const m = mountComponent(Kanji, { char: '繰', tone });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="kanji"]') as HTMLElement;
    for (const re of expected) expect(el.className).toMatch(re);
  });
});
