// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a read that failed is shown as a failure — not an empty page.
describe('Workspace Instruments /+page — a failed read', () => {
  it('shows the error with retry, not the empty page', () => {
    const m = mountComponent(Page, { data: { workspace: { id: 'p1', name: 'p' }, tools: [], mcpToolStats: [], services: [], error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.textContent).not.toContain('0 tools');
  });

  // #291: "Tools used" lives here now — the metric it hung off was retired.
  it('shows the tools the assistants used', () => {
    const m = mountComponent(Page, {
      data: {
        workspace: { id: 'p1', name: 'p' }, tools: [], mcpToolStats: [], services: [],
        toolUsage: [{ tool: 'Read', calls: 12, failed: 0, sessions: 3 }, { tool: 'Edit', calls: 5, failed: 1, sessions: 2 }], toolUsageError: null, error: null,
      } as unknown as PageData,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="tools-used"]')?.textContent).toContain('Read');
  });

  it('says when the tool usage could not be read', () => {
    const m = mountComponent(Page, {
      data: {
        workspace: { id: 'p1', name: 'p' }, tools: [], mcpToolStats: [], services: [],
        toolUsage: [], toolUsageError: 'Bad Gateway', error: null,
      } as unknown as PageData,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="tools-used-error"]')?.textContent).toContain('Bad Gateway');
  });
});
