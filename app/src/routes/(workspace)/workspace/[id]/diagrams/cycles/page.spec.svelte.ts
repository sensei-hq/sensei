// @vitest-environment jsdom
// Cycles was text only (user, 2026-10-28): each cycle now draws its units and the
// dependencies among them, beside the cut in words.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { payload } from '../layering-fixture.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

describe('Diagrams · Cycles', () => {
  it('draws each cycle as a graph of its units, with the cut still said in words', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(payload()), { status: 200 })));
    const m = mountComponent(Page, { data: { workspaceId: 'w-1' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-testid="cycle-0"]')).not.toBeNull());
    const card = m.container.querySelector('[data-testid="cycle-0"]')!;
    const diagram = card.querySelector('[data-testid="cycle-diagram-0"]');
    expect(diagram).not.toBeNull();
    expect(diagram!.querySelector('[data-graph-zoom-controls]')).not.toBeNull();
    expect(card.querySelector('[data-testid="cycle-cut-0"]')?.textContent).toMatch(/senseid\/db → senseid\/tasks/);
  });
});
