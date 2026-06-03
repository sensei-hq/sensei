// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import KanjiHeaderHarness from './KanjiHeader.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('KanjiHeader', () => {
  it('renders kanji, eyebrow, and title', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支',
      eyebrow: 'foundation',
      title: 'Checking components',
      withRight: false,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('支');
    expect(m.container.textContent).toContain('foundation');
    expect(m.container.textContent).toContain('Checking components');
  });

  it('eyebrow uses uppercase + ink-mute', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'foundation', title: 'X', withRight: false,
    });
    cleanup.push(m.destroy);
    const eyebrow = m.container.querySelector('[data-component="kanji-header-eyebrow"]') as HTMLElement;
    expect(eyebrow.className).toMatch(/\buppercase\b/);
    expect(eyebrow.className).toMatch(/\btext-ink-mute\b/);
  });

  it('kanji uses font-kanji + text-accent', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'x', title: 'X', withRight: false,
    });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="kanji-header-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\bfont-kanji\b/);
    expect(kanji.className).toMatch(/\btext-accent\b/);
  });

  it('renders the right slot when provided', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'x', title: 'X', withRight: true,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-test="harness-right"]')).toBeTruthy();
  });
});
