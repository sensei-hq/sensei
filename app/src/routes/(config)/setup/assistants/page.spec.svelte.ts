// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

class QuietEventSource {
  onopen: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 0;
  addEventListener() {}
  removeEventListener() {}
  close() {}
}

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const detectPosts = (spy: ReturnType<typeof vi.fn>) =>
  spy.mock.calls.filter(([url, init]) =>
    new URL(String(url)).pathname === '/api/assistants/detect' && (init as RequestInit | undefined)?.method === 'POST');

// #455: first run stores this machine's installs — the assistants stage posts the
// detection on entry, so Settings and the relay picker have rows to read.
describe('Setup · Assistants — on entry', () => {
  it('posts the detection once', async () => {
    vi.stubGlobal('EventSource', QuietEventSource);
    const fetchSpy = vi.fn(async () => new Response('[]', { status: 200 }));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(detectPosts(fetchSpy)).toHaveLength(1));
  });

  it('a refused detection says why', async () => {
    vi.stubGlobal('EventSource', QuietEventSource);
    vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 500, statusText: 'Internal Server Error' })));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="detect-error"]')?.textContent).toMatch(/500|Internal/),
    );
  });
});
