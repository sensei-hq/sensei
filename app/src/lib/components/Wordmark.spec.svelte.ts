// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import WordmarkHarness from './Wordmark.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Wordmark', () => {
  it('renders the sensei.svg mark and the lowercase word', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    const mark = m.container.querySelector('[data-component="wordmark-mark"]') as HTMLImageElement;
    expect(mark).not.toBeNull();
    expect(mark.getAttribute('src')).toBe('/sensei.svg');
    expect(m.container.textContent).toContain('sensei');
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
    const mark = m.container.querySelector('[data-component="wordmark-mark"]') as HTMLElement;
    expect(mark.className).toMatch(/\bh-5\b/);
  });

  it('applies lg size classes', () => {
    const m = mountComponent(WordmarkHarness, { size: 'lg' });
    cleanup.push(m.destroy);
    const mark = m.container.querySelector('[data-component="wordmark-mark"]') as HTMLElement;
    expect(mark.className).toMatch(/\bh-9\b/);
  });
});
