// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #288: a library list that could not be read is an error with retry — not
// "No libraries indexed."
describe('Libraries /+page — a failed read', () => {
  it('shows the error, not the empty state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' }));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.textContent).not.toContain('No libraries indexed.');
  });
});

// #341: a selected library shows what it provides — its own docs, skills and
// agents — read through the daemon.
describe('Libraries /+page — what a library provides', () => {
  it('lists the docs, skills and agents of the selected library', async () => {
    const libs = { total: 1, libs: [{ id: 'r', name: 'rokkit', ecosystem: 'npm', repoCount: 2, repos: ['app'], packages: ['@rokkit/ui'], skillCount: 1, agentCount: 1 }] };
    const answers: Record<string, unknown> = {
      '/api/libs?': libs,
      '/api/libs/r/usage': { usage: [] },
      '/api/libs/r/skills': [{ name: 'styling', focus: 'styling', body: '# Styling with rokkit' }],
      '/api/libs/r/agents': [{ name: 'rokkit-styles-reviewer', focus: 'styles', body: '# Review' }],
      '/api/libs/r/docs': [{ title: 'Button', component: 'Button', content: '# Button' }],
    };
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string) => {
      const key = Object.keys(answers).find((k) => new URL(url).pathname + (url.includes('?') && k.endsWith('?') ? '?' : '') === k);
      return key ? { ok: true, status: 200, json: async () => answers[key] } : { ok: false, status: 404, statusText: 'Not Found' };
    }));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('.lib-card')).not.toBeNull());
    (m.container.querySelector('.lib-card') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('rokkit-styles-reviewer'));
    const text = m.container.textContent ?? '';
    expect(text).toContain('Provided by the library');
    expect(text).toContain('styling');
    expect(text).toContain('Button');
    expect(text).toContain('@rokkit/ui');
  });
});
