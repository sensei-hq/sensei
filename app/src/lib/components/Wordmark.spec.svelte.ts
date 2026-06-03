// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import WordmarkHarness from './Wordmark.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Wordmark', () => {
  it('renders the kanji 先生 and the word Sensei', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('先生');
    expect(m.container.textContent).toContain('Sensei');
  });

  it('uses accent color for the kanji', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-accent\b/);
    expect(kanji.className).toMatch(/\bfont-kanji\b/);
  });

  it('uses the display font for the word', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    const word = m.container.querySelector('[data-component="wordmark-word"]') as HTMLElement;
    expect(word.className).toMatch(/\bfont-display\b/);
  });

  it('applies sm size classes', () => {
    const m = mountComponent(WordmarkHarness, { size: 'sm' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-lg\b/);
  });

  it('applies lg size classes', () => {
    const m = mountComponent(WordmarkHarness, { size: 'lg' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-3xl\b/);
  });
});
