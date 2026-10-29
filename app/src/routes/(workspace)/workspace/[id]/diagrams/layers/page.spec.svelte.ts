// @vitest-environment jsdom
// Layers (user, 2026-10-28): the wheel alone zoomed too fast to aim, and there was no
// on-canvas way to zoom. The diagram carries rokkit's overlay — violations, density and
// zoom (−, %, +) — as the schema diagrams do; the violations toggle is the overlay's, once.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { payload } from '../layering-fixture.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

describe('Diagrams · Layers', () => {
  it('draws the layering with the zoom overlay and one violations toggle', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(payload()), { status: 200 })));
    const m = mountComponent(Page, { data: { workspaceId: 'w-1' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="layers-diagram"]')).not.toBeNull());
    const zoom = m.container.querySelectorAll('[data-graph-zoom-controls]');
    expect(zoom).toHaveLength(1);
    expect([...zoom[0].querySelectorAll('[data-graph-zoom]')].map((b) => b.getAttribute('data-graph-zoom'))).toEqual(['out', 'reset', 'in']);
    expect(zoom[0].querySelector('[data-graph-zoom="reset"]')?.textContent?.trim()).toBe('100%');
    expect(m.container.querySelectorAll('[data-graph-violations]')).toHaveLength(1);
  });
});
