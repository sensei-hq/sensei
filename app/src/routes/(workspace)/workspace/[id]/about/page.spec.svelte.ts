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
// D87: phase B moves the page onto rokkit's `ScreenShell`; the `pending: L5
// about` reds below pin that adoption (the shell and its `workspace.about` view
// id, the header's title/description, the body's own scroll, the rail's glyph on
// the mark, no search) and the two Retries the Notices already promise.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { failureNotice } from '@sensei/ui';
import { KANJI_ICONS } from '$lib/glyph-vocabulary.js';
import type { DojoBindingSuggestion, DojoMembership } from '$lib/types.js';
import { SECTIONS } from '../workspace-sidebar-view.js';
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

// The failure shape `tryGet` / `tryPut` / `tryPost` read: the daemon's message
// reaches the reader through `statusText` (api.ts:147), so that is what a failed
// read or write answers here.
const failed = (status: number, statusText: string) => ({
  ok: false,
  status,
  statusText,
  json: async () => ({}),
  text: async () => '',
});

const REPOS = [
  { id: 'r1', name: 'daemon', path: '~/daemon' },
  { id: 'r2', name: 'app', path: '~/app' },
];

/** Route the page's reads by path; anything unlisted answers 404. */
const stubReads = (routes: Record<string, () => unknown>) => {
  const spy = vi.fn((input: RequestInfo | URL) => {
    const url = String(input);
    const hit = Object.keys(routes).find((path) => url.includes(path));
    return Promise.resolve(
      hit ? routes[hit]!() : { ok: false, status: 404, statusText: 'Not Found', json: async () => ({}) },
    );
  });
  vi.stubGlobal('fetch', spy);
  return spy;
};

// A read that has been released lands a few microtasks later; the answer it
// writes is only final once the flush has settled, so a stale-answer red waits
// for both rather than for the first tick (which can beat the write).
const settle = async () => {
  for (let i = 0; i < 5; i += 1) {
    await Promise.resolve();
    await tick();
  }
};

const retryButton = async (m: { container: HTMLElement }, notice: string) =>
  vi.waitFor(() => {
    const b = m.container.querySelector(
      `[${notice}] [data-action="retry"]`,
    ) as HTMLButtonElement | null;
    expect(b).not.toBeNull();
    return b as HTMLButtonElement;
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
  // N1: the patch is partial in both directions — a field this workspace has
  // left empty is not in the body at all, so the daemon's COALESCE keeps what
  // it stores instead of writing an empty string over it.
  it('leaves an unset field out of the patch', async () => {
    const fetchSpy = vi.fn().mockResolvedValue(okJson({}));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mount({ workspace: workspace({ description: '' }) });
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
    const body = JSON.parse(String((write?.[1] as RequestInit).body));
    expect(body.client).toBe('Globex');
    expect(body).not.toHaveProperty('description');
  });
});

// D83: a failed write must not look like success. The state modules pin their
// own lifecycle; these pin that the page *draws* it, which phase B rewrites.
describe('Workspace · About — a failed write is said', () => {
  it('a save the daemon refused says why, and never “saved”', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(failed(500, 'Internal Server Error')));
    const m = mount();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="metadata-client"]')).not.toBeNull(),
    );

    const client = m.container.querySelector('[data-testid="metadata-client"]') as HTMLInputElement;
    client.value = 'Globex';
    client.dispatchEvent(new Event('input', { bubbles: true }));

    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="metadata-save-status"]')?.textContent).toBe(
        'Internal Server Error',
      ),
    );
    expect(m.container.querySelector('[data-testid="metadata-save-status"]')?.textContent).not.toBe('saved');
  });

  it('a Confirm the daemon refused says why, and the inferred binding stays', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(failed(502, 'Bad Gateway')));
    const m = mount({ suggestion: suggestion() });
    const button = await vi.waitFor(() => {
      const b = m.container.querySelector('[data-testid="binding-confirm"]') as HTMLButtonElement | null;
      expect(b).not.toBeNull();
      return b as HTMLButtonElement;
    });

    button.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-error]')?.textContent).toContain('Bad Gateway'),
    );
    expect(m.container.querySelector('[data-binding-state="inferred"]')).not.toBeNull();
    expect(m.container.querySelector('[data-binding-state="confirmed"]')).toBeNull();
    expect(
      (m.container.querySelector('[data-testid="binding-confirm"]') as HTMLButtonElement).disabled,
    ).toBe(false);
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
// Disabled until phase B; each fails today for a reason of its own (the shell's
// hooks are absent). The hooks are rokkit's own — @rokkit/app 1.15.1,
// ScreenShell.svelte: `data-screen-shell` with `data-view`, `-title`,
// `-description`, `-body`, `-mark`, `-search` — and the shell is spied on too, so
// a hand-rolled header carrying the same attributes cannot pass for it.

describe('Workspace · About — the shell (phase B)', () => {
  it.skip('pending: L5 about — renders rokkit’s ScreenShell with the workspace.about view id', async () => {
    const m = mount();
    await vi.waitFor(() => expect(vi.mocked(ScreenShell).mock.calls.length).toBeGreaterThan(0));
    const shell = m.container.querySelector('[data-screen-shell]') as HTMLElement | null;
    expect(shell).not.toBeNull();
    expect(shell?.getAttribute('data-view')).toBe('workspace.about');
    // Svelte 5 calls a component as (anchor, props): the props are the second arg.
    // Reading the anchor instead is unpassable — `viewId` is never on it.
    const props = vi.mocked(ScreenShell).mock.calls[0]![1] as unknown as { viewId?: string };
    expect(props).toMatchObject({ viewId: 'workspace.about' });
  });

  it.skip('pending: L5 about — the shell header carries this workspace’s name and goal', async () => {
    const m = mount();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-title]')?.textContent?.trim()).toBe('Sensei'),
    );
    expect(m.container.querySelector('[data-screen-description]')?.textContent?.trim()).toBe(
      'Ship the daemon',
    );

    // A second workspace, so a title hard-coded to the fixture cannot pass.
    const other = mount({
      workspace: workspace({ name: 'Globex HQ', goal: 'Cut the flaky suite' }),
    });
    await vi.waitFor(() =>
      expect(other.container.querySelector('[data-screen-title]')?.textContent?.trim()).toBe('Globex HQ'),
    );
    expect(other.container.querySelector('[data-screen-description]')?.textContent?.trim()).toBe(
      'Cut the flaky suite',
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
    expect(body.querySelectorAll('.repo-row').length).toBe(2);
    expect(body.querySelector('[data-screen-header]')).toBeNull();
  });

  // L0: the mark is the rail's own glyph — the entry that opened this screen.
  // A reader who clicked 情 must find 情, not a new glyph invented for the title.
  it.skip('pending: L5 about — the mark is the rail’s glyph for About, in the reader’s mode', async () => {
    const rail = SECTIONS.find((s) => s.id === 'about')!;
    const iconClass = KANJI_ICONS[rail.kanji] as string;
    expect(rail.kanji).toBe('情');
    expect(iconClass).toBeTruthy();

    const kanji = mount();
    const mark = await vi.waitFor(() => {
      const el = kanji.container.querySelector('[data-screen-mark]') as HTMLElement | null;
      expect(el).not.toBeNull();
      return el as HTMLElement;
    });
    expect(mark.textContent?.trim()).toBe(rail.kanji);

    glyphs.mode = 'icon';
    const icon = mount();
    await vi.waitFor(() => {
      const el = icon.container.querySelector('[data-screen-mark]') as HTMLElement | null;
      expect(el?.className).toContain(iconClass);
    });
  });

  // L0: About is a form and a short list — nothing to search for, so the shell
  // is told `search={false}` and draws none.
  it.skip('pending: L5 about — draws no search: there is nothing here to search for', async () => {
    const m = mount();
    const shell = await vi.waitFor(() => {
      const s = m.container.querySelector('[data-screen-shell]') as HTMLElement | null;
      expect(s).not.toBeNull();
      return s as HTMLElement;
    });
    expect(shell.querySelector('[data-screen-search]')).toBeNull();
  });
});

// ── Red (phase B): the Notices' remedy is a Retry that re-reads ────────────
//
// `failureNotice` already tells the reader "Retry", and both failed reads must
// offer the button. Retry re-reads through the same function the loader uses (one
// copy of each read — no `invalidateAll()`, which would re-run every load on the
// route), guards a second press while one is in flight, and drops an answer that
// lands after a navigation.

describe('Workspace · About — the failed reads offer a Retry (phase B)', () => {
  const reposNotice = failureNotice(
    "Couldn't read this workspace's repositories",
    'daemon unreachable',
  );
  const reposDown = () => ({
    repos: [],
    reposError: 'daemon unreachable',
    reposNotice,
  });

  it.skip('pending: L5 about — a repos Retry re-reads, and the list comes back', async () => {
    const spy = stubReads({ '/api/workspaces/w1/repos': () => okJson({ repos: REPOS }) });
    const m = mount(reposDown());
    const button = await retryButton(m, 'data-repos-error');
    button.click();

    await vi.waitFor(() => expect(m.container.textContent).toContain('Repos (2)'));
    expect(m.container.querySelectorAll('.repo-row').length).toBe(2);
    expect(m.container.querySelector('[data-repos-error]')).toBeNull();
    // Only the repos: a repos Retry does not re-read the binding.
    expect(spy).toHaveBeenCalledTimes(1);
    expect(String(spy.mock.calls[0]![0])).toContain('/api/workspaces/w1/repos');
  });

  it.skip('pending: L5 about — a repos Retry that fails again says the new reason', async () => {
    stubReads({ '/api/workspaces/w1/repos': () => failed(502, 'Bad Gateway') });
    const m = mount(reposDown());
    const button = await retryButton(m, 'data-repos-error');
    button.click();

    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-repos-error]')?.textContent).toContain('Bad Gateway'),
    );
    expect(m.container.querySelector('[data-repos-error]')?.textContent).not.toContain(
      'daemon unreachable',
    );
    expect(m.container.textContent).not.toContain('Repos (0)');
    expect(m.container.querySelectorAll('.repo-row').length).toBe(0);
  });

  it.skip('pending: L5 about — a binding Retry re-reads the pair, and the chip comes back', async () => {
    const spy = stubReads({
      '/dojo-suggestion': () => okJson({ suggestion: suggestion() }),
      '/api/dojo/memberships': () => okJson([]),
    });
    const m = mount({ bindingError: 'daemon unreachable' });
    const button = await retryButton(m, 'data-binding-load-error');
    button.click();

    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-state="inferred"]')).not.toBeNull(),
    );
    expect(m.container.querySelector('[data-binding-load-error]')).toBeNull();
    // The loader's own pair, and nothing else (no repos re-read).
    expect(spy).toHaveBeenCalledTimes(2);
    const urls = spy.mock.calls.map(([input]) => String(input));
    expect(urls.some((u) => u.includes('/dojo-suggestion'))).toBe(true);
    expect(urls.some((u) => u.includes('/api/dojo/memberships'))).toBe(true);
  });

  it.skip('pending: L5 about — a binding Retry that fails again is the error, never “stay local”', async () => {
    stubReads({
      '/dojo-suggestion': () => failed(502, 'Bad Gateway'),
      '/api/dojo/memberships': () => failed(502, 'Bad Gateway'),
    });
    const m = mount({ bindingError: 'daemon unreachable' });
    const button = await retryButton(m, 'data-binding-load-error');
    button.click();

    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-load-error]')?.textContent).toContain(
        'Bad Gateway',
      ),
    );
    expect(m.container.querySelector('[data-binding-load-error]')?.textContent).not.toContain(
      'daemon unreachable',
    );
    expect(m.container.textContent).not.toContain('stay local');
  });

  it.skip('pending: L5 about — two presses of the repos Retry are one read', async () => {
    let release: (v: unknown) => void = () => {};
    const spy = vi.fn((input: RequestInfo | URL) =>
      String(input).includes('/api/workspaces/w1/repos')
        ? new Promise((resolve) => {
            release = resolve as (v: unknown) => void;
          })
        : Promise.resolve({ ok: false, status: 404, statusText: 'Not Found', json: async () => ({}) }),
    );
    vi.stubGlobal('fetch', spy);
    const m = mount(reposDown());
    const button = await retryButton(m, 'data-repos-error');
    button.click();
    button.click();

    expect(spy).toHaveBeenCalledTimes(1);
    release(okJson({ repos: REPOS }));
    await vi.waitFor(() => expect(m.container.textContent).toContain('Repos (2)'));
  });

  it.skip('pending: L5 about — two presses of the binding Retry are one pair of reads', async () => {
    const waiters: Array<{ url: string; release: (v: unknown) => void }> = [];
    const spy = vi.fn(
      (input: RequestInfo | URL) =>
        new Promise((resolve) => {
          waiters.push({ url: String(input), release: resolve as (v: unknown) => void });
        }),
    );
    vi.stubGlobal('fetch', spy);
    const m = mount({ bindingError: 'daemon unreachable' });
    const button = await retryButton(m, 'data-binding-load-error');
    button.click();
    button.click();

    expect(spy).toHaveBeenCalledTimes(2);
    expect(waiters.filter((w) => w.url.includes('/dojo-suggestion')).length).toBe(1);
    expect(waiters.filter((w) => w.url.includes('/api/dojo/memberships')).length).toBe(1);

    for (const w of waiters) {
      w.release(w.url.includes('/dojo-suggestion') ? okJson({ suggestion: suggestion() }) : okJson([]));
    }
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-binding-state="inferred"]')).not.toBeNull(),
    );
  });

  it.skip('pending: L5 about — a repos Retry that lands after a navigation is dropped', async () => {
    let release: (v: unknown) => void = () => {};
    vi.stubGlobal(
      'fetch',
      vi.fn(() => new Promise((resolve) => {
        release = resolve as (v: unknown) => void;
      })),
    );
    const props = $state({ data: data(reposDown()) });
    const m = mountComponent(Page, props as never);
    cleanup.push(m.destroy);
    const button = await retryButton(m, 'data-repos-error');
    button.click();

    props.data = data({
      repos: [{ id: 'n1', name: 'from_the_url', path: '~/n1' }],
      reposError: null,
      reposNotice: null,
    });
    await tick();

    release(okJson({ repos: REPOS }));
    await settle();

    // The navigation's workspace is what the reader is looking at: its one repo,
    // and no error. The retried answer (the other workspace's two repos, and no
    // notice) must not have landed on top of it.
    const rows = [...m.container.querySelectorAll('.repo-row')].map((r) => r.textContent ?? '');
    expect(rows.length).toBe(1);
    expect(rows[0]).toContain('from_the_url');
    expect(m.container.querySelector('[data-repos-error]')).toBeNull();
  });
});
