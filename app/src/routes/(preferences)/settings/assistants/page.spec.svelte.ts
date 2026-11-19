// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const today = new Date().toISOString();
const row = {
  family: 'claude', name: 'Claude Code', kind: 'assistant', version: '2.1',
  installed: true, configured: true, detectedAt: today,
};

// #455: Settings reads the installs detection stored, says when they were
// detected, and offers "Detect again" — which stores a fresh probe.
describe('Settings · Assistants — the stored installs', () => {
  it('names each tool and says when it was detected', async () => {
    vi.stubGlobal('fetch', vi.fn());
    const m = mountComponent(Page, { data: { assistants: [row], error: null } } as never);
    cleanup.push(m.destroy);
    const item = m.container.querySelector('[data-assistant="claude"]');
    expect(item?.textContent).toContain('Claude Code');
    expect(item?.querySelector('[data-detected-at]')?.textContent).toContain('detected today');
  });

  it('"Detect again" posts the detection', async () => {
    const fetchSpy = vi.fn(async () => new Response('[]', { status: 200 }));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { assistants: [row], error: null } } as never);
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-action="detect-again"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(1));
    const [url, init] = fetchSpy.mock.calls[0] as unknown as [string, RequestInit];
    expect(new URL(url).pathname).toBe('/api/assistants/detect');
    expect(init.method).toBe('POST');
  });

  it('a refused "Detect again" says why', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 500, statusText: 'Internal Server Error' })));
    const m = mountComponent(Page, { data: { assistants: [row], error: null } } as never);
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-action="detect-again"]') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="detect-error"]')?.textContent).toMatch(/500|Internal/),
    );
  });

  it('a failed read is the error state with Retry, not "No assistants detected"', async () => {
    vi.stubGlobal('fetch', vi.fn());
    const m = mountComponent(Page, { data: { assistants: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull();
    expect(m.container.textContent).not.toContain('No assistants detected');
  });
});
