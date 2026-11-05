// @vitest-environment jsdom
// Workspace · About — the screen as the reader sees it.
//
// The reads are the loader's (`+page.ts`): the component draws `data` and reads
// nothing of its own. The identity form auto-saves through its state; the dōjō
// binding resolves to confirmed | inferred | empty | error, and a failed read is
// the error, never "no binding — stays local". The e2e `dojo-binding.spec.ts`
// opens this screen by `[data-testid="workspace-bindings"]`, so that hook is a
// contract, not a convenience.
//
// D87: phase B moves the page onto rokkit's `ScreenShell`; the five
// `pending: L5 about` reds below pin that adoption (the shell and its view id,
// the header's title/description, the body's own scroll, the mark's glyph mode).
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { failureNotice } from '@sensei/ui';
import type { DojoBindingSuggestion, DojoMembership } from '$lib/types.js';
import Page from './+page.svelte';

vi.mock('$app/state', () => ({
  page: { params: { id: 'w1' }, url: new URL('http://localhost/workspace/w1/about') },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

// D87: the DOM hooks alone can't tell rokkit's shell from a hand-rolled header
// carrying the same attributes, so the shell itself is asked for this view's id.
vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

// The shell draws the mark in the reader's glyph mode, so the mode is the one
// thing this file stubs — every other test reads the app's default.
const glyphs = vi.hoisted(() => ({ mode: 'kanji' as 'kanji' | 'icon' }));
vi.mock('$lib/glyphs', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, useGlyphMode: () => () => glyphs.mode };
});

import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  vi.mocked(ScreenShell).mockClear();
  glyphs.mode = 'kanji';
});

const MEMBERSHIP_ID = '11111111-2222-4333-8444-555555555555';

const membership = (over: Partial<DojoMembership> = {}): DojoMembership => ({
  id: MEMBERSHIP_ID,
  registry_url: 'https://dojo.acme.internal',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  kind: 'client',
  role: 'contributor',
  authenticated_via: 'device_code',
  attribution_default: 'named',
  org_slugs: ['acme'],
  sync_status: 'healthy',
  last_seq: 0,
  last_heartbeat_at: null,
  enabled: true,
  bound_workspaces: [],
  persona_slot: null,
  ...over,
});

const suggestion = (over: Partial<DojoBindingSuggestion> = {}): DojoBindingSuggestion => ({
  membership_id: MEMBERSHIP_ID,
  kind: 'client',
  matched_slug: 'acme',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  ...over,
});

const workspace = (over: Record<string, unknown> = {}) => ({
  id: 'w1',
  name: 'Sensei',
  client: 'Acme',
  goal: 'Ship the daemon',
  description: 'one workspace, one goal',
  maturity: 'active',
  ...over,
});

type Data = Parameters<typeof Page>[0] extends { data: infer D } ? D : never;

const data = (over: Record<string, unknown> = {}): Data =>
  ({
    workspace: workspace(),
    repos: [
      { id: 'r1', name: 'daemon', path: '~/daemon' },
      { id: 'r2', name: 'app', path: '~/app' },
    ],
    reposError: null,
    reposNotice: null,
    suggestion: null,
    memberships: [],
    bindingError: null,
    ...over,
  }) as Data;

const mount = (over: Record<string, unknown> = {}) => {
  const m = mountComponent(Page, { data: data(over) } as never);
  cleanup.push(m.destroy);
  return m;
};

const okJson = (body: unknown) => ({
  ok: true,
  status: 200,
  json: async () => body,
  text: async () => JSON.stringify(body),
});

// ── Green: today's page, and the contract phase B has to keep ────────────────

describe('Workspace · About — the loader’s workspace', () => {
  it('draws the name, the goal and the seeded identity fields', async () => {
    const m = mount();
    await vi.waitFor(() => expect(m.container.textContent).toContain('Sensei'));
    expect((m.container.querySelector('[data-testid="metadata-client"]') as HTMLInputElement).value).toBe('Acme');
    expect((m.container.querySelector('[data-testid="metadata-goal"]') as HTMLTextAreaElement).value).toBe(
      'Ship the daemon',
    );
    expect((m.container.querySelector('[data-testid="metadata-description"]') as HTMLTextAreaElement).value).toBe(
      'one workspace, one goal',
    );
  });

  it('makes no read of its own on mount', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(okJson({}));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mount();
    await vi.waitFor(() => expect(m.container.textContent).toContain('Sensei'));
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('saves an edit as a partial patch and says it saved', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(okJson({}));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mount();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="metadata-client"]')).not.toBeNull(),
    );

    const client = m.container.querySelector('[data-testid="metadata-client"]') as HTMLInputElement;
    client.value = 'Globex';
    client.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="metadata-save-status"]')?.textContent).toBe('saved'),
    );
    const write = fetchSpy.mock.calls.find(([, init]) => (init as RequestInit | undefined)?.method === 'PUT');
    expect(write).toBeDefined();
    expect(String(write?.[0])).toContain('/api/workspaces/w1');
    const body = JSON.parse(String((write?.[1] as RequestInit).body));
    expect(body.client).toBe('Globex');
    expect(body.goal).toBe('Ship the daemon');
  });

  it('lists the repos, with the count in the heading', async () => {
    const m = mount();
    await vi.waitFor(() => expect(m.container.querySelectorAll('.repo-row').length).toBe(2));
    expect(m.container.textContent).toContain('Repos (2)');
    expect(m.container.textContent).toContain('daemon');
  });

  // #262: a failed repos read is the error, never "(0)" and an empty list.
  it('a failed repos read is the error, not a count of zero', async () => {
    const m = mount({
      repos: [],
      reposError: 'Bad Gateway',
      reposNotice: failureNotice("Couldn't read this workspace's repositories", 'Bad Gateway'),
    });
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-repos-error]')?.textContent).toContain('Bad Gateway'),
    );
    expect(m.container.textContent).not.toContain('Repos (0)');
    expect(m.container.querySelectorAll('.repo-row').length).toBe(0);
  });

  it('chips the stack’s languages and frameworks', async () => {
    const m = mount({
      workspace: workspace({ stack: { languages: ['Rust'], frameworks: ['Svelte'] } }),
    });
    await vi.waitFor(() => expect(m.container.textContent).toContain('Stack'));
    const text = m.container.textContent ?? '';
    expect(text).toContain('Rust');
    expect(text).toContain('Svelte');
  });
});

describe('Workspace · About — the dōjō binding', () => {
  it('a bound workspace reads the membership’s tenant key and url as confirmed', async () => {
    const m = mount({
      workspace: workspace({ dojo_id: MEMBERSHIP_ID }),
      memberships: [membership()],
    });
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-state="confirmed"]')).not.toBeNull(),
    );
    const section = m.container.querySelector('[data-testid="workspace-bindings"]') as HTMLElement;
    expect(section.textContent).toContain('github/acme');
    expect(section.textContent).toContain('https://dojo.acme.internal/github/acme');
  });

  it('an inferred binding offers Confirm, and confirming swaps in the confirmed row', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(okJson({ ok: true, dojo_id: MEMBERSHIP_ID })),
    );
    const m = mount({ suggestion: suggestion() });
    const button = await vi.waitFor(() => {
      const b = m.container.querySelector('[data-testid="binding-confirm"]') as HTMLButtonElement;
      expect(b).not.toBeNull();
      return b;
    });
    expect(m.container.querySelector('[data-binding-state="inferred"]')).not.toBeNull();

    button.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-state="confirmed"]')).not.toBeNull(),
    );
    expect(m.container.querySelector('[data-binding-state="inferred"]')).toBeNull();
  });

  // #262: a binding that could not be read is the error, never "stays local".
  it('a failed binding read is the error, not “no binding”', async () => {
    const m = mount({ bindingError: 'daemon unreachable' });
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-load-error]')?.textContent).toContain(
        'daemon unreachable',
      ),
    );
    expect(m.container.textContent).not.toContain('stay local');
  });

  it('with nothing inferred or bound, says the findings stay local', async () => {
    const m = mount();
    await vi.waitFor(() => expect(m.container.textContent).toContain('stay local'));
    expect(m.container.querySelector('[data-binding-state]')).toBeNull();
  });
});

// ── Red (phase B): the page moves onto rokkit's ScreenShell ──────────────────
//
// These four are disabled until phase B; each fails today for its own reason.

describe('Workspace · About — the shell (phase B)', () => {
  it.skip('pending: L5 about — renders rokkit’s ScreenShell with the workspace.about view id', async () => {
    const m = mount();
    await vi.waitFor(() => expect(vi.mocked(ScreenShell).mock.calls.length).toBeGreaterThan(0));
    const shell = m.container.querySelector('[data-screen-shell]') as HTMLElement | null;
    expect(shell).not.toBeNull();
    expect(shell?.getAttribute('data-view')).toBe('workspace.about');
    const props = vi.mocked(ScreenShell).mock.calls[0][0] as unknown as { viewId?: string };
    expect(props.viewId).toBe('workspace.about');
  });

  it.skip('pending: L5 about — the shell header carries the name and the goal', async () => {
    const m = mount();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-title]')?.textContent?.trim()).toBe('Sensei'),
    );
    expect(m.container.querySelector('[data-screen-description]')?.textContent?.trim()).toBe(
      'Ship the daemon',
    );
  });

  it.skip('pending: L5 about — the goal falls back to the list payload’s vision', async () => {
    const m = mount({ workspace: workspace({ goal: null, vision: 'the vision line' }) });
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-description]')?.textContent?.trim()).toBe(
        'the vision line',
      ),
    );
  });

  it.skip('pending: L5 about — the body scrolls under the header, and holds every section', async () => {
    const m = mount();
    const body = await vi.waitFor(() => {
      const b = m.container.querySelector('[data-screen-body]') as HTMLElement | null;
      expect(b).not.toBeNull();
      return b as HTMLElement;
    });
    for (const hook of ['workspace-metadata', 'workspace-bindings']) {
      expect(body.querySelector(`[data-testid="${hook}"]`)).not.toBeNull();
    }
    expect(body.querySelector('[data-screen-header]')).toBeNull();
  });

  it.skip('pending: L5 about — the mark follows the glyph preference', async () => {
    const kanji = mount();
    const mark = await vi.waitFor(() => {
      const el = kanji.container.querySelector('[data-screen-mark]') as HTMLElement | null;
      expect(el).not.toBeNull();
      return el as HTMLElement;
    });
    expect(mark.textContent?.trim()).toMatch(/[\u4e00-\u9fff]/);

    glyphs.mode = 'icon';
    const icon = mount();
    await vi.waitFor(() => {
      const el = icon.container.querySelector('[data-screen-mark]') as HTMLElement | null;
      expect(el?.className).toContain('i-solar-');
    });
  });
});
