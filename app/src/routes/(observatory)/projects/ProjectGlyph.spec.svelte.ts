// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './ProjectGlyph.harness.svelte';
import type { ProjectIcon } from './buckets.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

function mount(icon: ProjectIcon) {
  const m = mountComponent(Harness, { icon });
  cleanup.push(m.destroy);
  return m;
}

const img = (m: { container: HTMLElement }) => m.container.querySelector('img');
const kanji = (m: { container: HTMLElement }) => m.container.querySelector('span.kanji');

describe('ProjectGlyph', () => {
  it('renders the image with its src and no kanji for an image icon', () => {
    const m = mount({ kind: 'image', src: 'http://127.0.0.1:7744/api/projects/p/icon', glyph: '場' });
    expect(img(m)?.getAttribute('src')).toBe('http://127.0.0.1:7744/api/projects/p/icon');
    expect(kanji(m)).toBeNull();
  });

  it('renders the kanji glyph and no image for a kanji icon', () => {
    const m = mount({ kind: 'kanji', glyph: '禅' });
    expect(kanji(m)?.textContent).toBe('禅');
    expect(img(m)).toBeNull();
  });

  it('falls back to the kanji glyph when the image fails to load', async () => {
    const m = mount({ kind: 'image', src: 'http://127.0.0.1:7744/api/projects/p/icon', glyph: '場' });
    const el = img(m);
    expect(el).not.toBeNull();
    // Simulate a broken image (e.g. a project whose icon 404s) — the component
    // swaps to the kanji fallback rather than showing a broken image.
    el!.dispatchEvent(new Event('error'));
    await Promise.resolve();
    expect(img(m)).toBeNull();
    expect(kanji(m)?.textContent).toBe('場');
  });
});
