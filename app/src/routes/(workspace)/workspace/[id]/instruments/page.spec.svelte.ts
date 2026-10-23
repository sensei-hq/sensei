// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

vi.mock('$app/state', () => ({ page: { params: { id: 'p1' } } }));
vi.mock('$app/navigation', () => ({ invalidateAll: vi.fn(async () => {}) }));

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

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

  // #304: a refused toggle used to snap back silently.
  it('a service toggle the daemon refused says why on its row, and keeps the switch', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 503, statusText: 'Service Unavailable' })));
    const m = mountComponent(Page, {
      data: {
        workspace: { id: 'p1', name: 'p' }, tools: [], mcpToolStats: [], toolUsage: [], toolUsageError: null, error: null,
        services: [{ id: 'svc', name: 'gh', displayName: 'GitHub', protocol: 'mcp', kind: 'server', toolsCount: 0, enabledForWorkspace: true }],
      } as unknown as PageData,
    });
    cleanup.push(m.destroy);
    const toggle = m.container.querySelector<HTMLButtonElement>('[data-testid="service-toggle-gh"]')!;
    toggle.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="service-toggle-error-gh"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(toggle.textContent).toBe('On');
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
