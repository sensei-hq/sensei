// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #287: rework shows as ONE anti-pattern; opening it lists its files by
// top-level directory.
describe('Patterns /+page — rework hotspots', () => {
  it('lists the hotspot files by directory when opened', () => {
    const hotspots = {
      id: 'rework-hotspots', name: 'rework hotspots', kind: 'anti', isAntiPattern: true, lifecycle: 'suggested',
      confidence: 1, instanceCount: 3, ftrDelta: null,
      description: '3 files were re-edited heavily within a single session — by directory: crates 2, docs 1.',
      example: 'crates/a.rs, crates/b.rs, docs/x.md',
      groups: [{ directory: 'crates', files: ['crates/a.rs', 'crates/b.rs'] }, { directory: 'docs', files: ['docs/x.md'] }],
    };
    const m = mountComponent(Page, {
      data: { project: { id: 'p', name: 'p' }, followed: [], antiPatterns: [hotspots], error: null } as unknown as PageData,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelectorAll('[data-testid^="pattern-row-"]').length).toBe(1);
    (m.container.querySelector('[data-testid="pattern-toggle-rework-hotspots"]') as HTMLButtonElement).click();
    flushSync();
    const groups = [...m.container.querySelectorAll('[data-testid^="pattern-group-"]')].map((g) => [
      g.getAttribute('data-testid'),
      [...g.querySelectorAll('li')].map((li) => li.textContent?.trim()),
    ]);
    expect(groups).toEqual([
      ['pattern-group-crates', ['crates/a.rs', 'crates/b.rs']],
      ['pattern-group-docs', ['docs/x.md']],
    ]);
  });
});
