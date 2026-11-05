// @vitest-environment jsdom
// Settings · Dōjō connection (user, 2026-11-03): identities first; a connected
// identity lists its dōjōs and the repositories each has filed; every action for
// the selected identity sits in its header; failures are Notices.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { personaList } from '$lib/personas.svelte.js';
import Page from './+page.svelte';
import { membership, repo } from './dojo-connection.fixtures.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const json = (body: unknown) => ({ ok: true, status: 200, statusText: 'OK', json: async () => body });
const personas = {
  default: 'jerrythomas',
  personas: [
    { label: 'jerrythomas', githubLogin: 'jerrythomas', sessionSlot: 'jerrythomas', connected: true, isDefault: true, forgeToken: { state: 'active', expiresAt: 4_000_000_000 }, renewalDue: false, keychain: 'readable' },
    { label: 'jt-oss', githubLogin: null, sessionSlot: null, connected: false, isDefault: false, forgeToken: { state: 'unknown', expiresAt: null }, renewalDue: false, keychain: null },
  ],
};
const memberships = [
  membership({ id: 'p', kind: 'personal', tenant_key: 'github/jerrythomas', org_slugs: ['jerrythomas'] }),
  membership({ id: 'o', tenant_key: 'github/sensei-hq', org_slugs: ['sensei-hq'] }),
];
const entities = [
  repo({ entity_key: 'github.com/sensei-hq/sensei' }),
  repo({ entity_key: 'github.com/sensei-hq/web', state: 'skipped', last_error: 'not_subscribed', remedy: 'Private repositories sync on the Org plan.' }),
  repo({ entity_key: 'github.com/jerrythomas/burn-e' }),
  repo({ entity: 'forge_mirror', entity_key: 'jerrythomas', direction: 'pull', state: 'error', last_error: 'could not reach', synced_at: null }),
];
const data = (over: Record<string, unknown> = {}) => ({
  sync: { count: entities.length, counts: {}, entities }, error: null, memberships, membershipsError: null, ...over,
});

async function mount(d = data()) {
  vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string) => json(url.includes('/api/auth/personas') ? personas : {})));
  await personaList.load();
  const m = mountComponent(Page, { data: d });
  cleanup.push(m.destroy);
  await vi.waitFor(() => expect(m.container.querySelectorAll('[data-identity]').length).toBe(2));
  return m;
}
const q = (m: { container: HTMLElement }, sel: string) => m.container.querySelector(sel) as HTMLElement | null;
const all = (m: { container: HTMLElement }, sel: string) => [...m.container.querySelectorAll(sel)] as HTMLElement[];

describe('Settings · Dōjō connection', () => {
  it('lists identities by name, the default marked, the default selected', async () => {
    const m = await mount();
    expect(all(m, '[data-identity]').map((c) => c.dataset.identity)).toEqual(['jerrythomas', 'jt-oss']);
    expect(q(m, '[data-identity="jerrythomas"] [data-default-persona]')).not.toBeNull();
    expect(q(m, '[data-identity="jerrythomas"]')!.getAttribute('aria-pressed')).toBe('true');
  });

  it('identity cards carry no actions — those are in the header', async () => {
    const m = await mount();
    expect(all(m, '[data-identity] [data-action]')).toHaveLength(0);
    expect(q(m, '[data-identity-header] [data-action="sign-out"]')).not.toBeNull();
  });

  it('a connected identity shows its dōjōs, each with its repositories', async () => {
    const m = await mount();
    expect(all(m, '[data-dojo]').map((d) => d.dataset.dojo)).toEqual(['p', 'o']);
    expect(all(m, '[data-dojo="o"] [data-repository]').map((r) => r.dataset.repository)).toEqual([
      'github.com/sensei-hq/web', 'github.com/sensei-hq/sensei',
    ]);
  });

  it("a skipped repository's reason is a refusal Notice in the remedy's words", async () => {
    const m = await mount();
    const n = q(m, '[data-repository="github.com/sensei-hq/web"] [data-component="notice"]')!;
    expect(n.dataset.tone).toBe('refusal');
    expect(n.textContent).toContain('Private repositories sync on the Org plan.');
  });

  it('what travels both ways is shown once, above the identities', async () => {
    const m = await mount();
    expect(all(m, '[data-travels] [data-direction]').map((d) => d.dataset.direction)).toEqual(['out', 'both', 'in']);
  });

  // Text takes its surface's pair: ink on paper, on-accent-soft on the tint. One pair for every
  // tile read ~1:1 on whichever surface it was not made for (dark mode, 2026-11-04).
  it("each travels tile's text is the pair for its own surface", async () => {
    const m = await mount();
    for (const tile of all(m, '[data-travels] [data-direction]')) {
      const onTint = tile.classList.contains('bg-accent-soft');
      const texts = [...tile.querySelectorAll('[class*="text-"]')].map((e) => e.className);
      expect(onTint).toBe(tile.dataset.direction === 'in');
      for (const c of texts) {
        if (onTint) expect(c, tile.dataset.direction).toMatch(/text-on-accent-soft/);
        else expect(c, tile.dataset.direction).not.toMatch(/text-on-accent-soft/);
      }
    }
  });

  it('the filter narrows the repositories', async () => {
    const m = await mount();
    (q(m, '[data-filter="off"]') as HTMLButtonElement).click();
    flushSync();
    expect(all(m, '[data-repository]').map((r) => r.dataset.repository)).toEqual(['github.com/sensei-hq/web']);
  });

  it('selecting an identity that never connected shows how to connect, and Connect in the header', async () => {
    const m = await mount();
    (q(m, '[data-identity="jt-oss"]') as HTMLButtonElement).click();
    flushSync();
    expect(q(m, '[data-connect-steps]')).not.toBeNull();
    expect(q(m, '[data-identity-header] [data-action="primary"]')!.textContent).toContain('Connect with GitHub');
    expect(all(m, '[data-dojo]')).toHaveLength(0);
  });

  it("an identity's own sync failure (its forge mirror) is a fault Notice, not lost", async () => {
    const m = await mount();
    const n = q(m, '[data-identity-issues] [data-component="notice"]')!;
    expect(n.dataset.tone).toBe('fault');
    expect(n.textContent).toContain('could not reach');
  });

  it('a failed membership read is a fault Notice, never "no dōjō"', async () => {
    const m = await mount(data({ memberships: null, membershipsError: 'Failed to fetch' }));
    const n = q(m, '[data-load-notices] [data-component="notice"]')!;
    expect(n.dataset.tone).toBe('fault');
    expect(n.textContent).toContain("Couldn't load your dōjōs");
    expect(q(m, '[data-no-dojos]')).toBeNull();
  });

  it('a failed membership read shows no "0 dōjōs", no "Not filed" group, no summary', async () => {
    const m = await mount(data({ memberships: null, membershipsError: 'Failed to fetch' }));
    expect(q(m, '[data-dojo="unfiled"]')).toBeNull();
    expect(q(m, '[data-summary]')).toBeNull();
    expect(m.container.textContent).not.toMatch(/0 dōjōs/);
  });

  it('a failed sync read shows the dōjōs without "0 of 0" or "Nothing in this view"', async () => {
    const m = await mount(data({ sync: null, error: 'boom' }));
    expect(all(m, '[data-dojo]').map((d) => d.dataset.dojo)).toEqual(['p', 'o']);
    expect(m.container.textContent).not.toMatch(/0 of 0/);
    expect(q(m, '[data-dojo-empty]')).toBeNull();
    expect(q(m, '[data-summary]')).toBeNull();
  });

  it('two rows with the same key do not freeze the screen (2026-10-28)', async () => {
    const dup = [...entities, repo({ entity_key: 'github.com/sensei-hq/sensei' })];
    const m = await mount(data({ sync: { count: dup.length, counts: {}, entities: dup } }));
    expect(all(m, '[data-dojo="o"] [data-repository]')).toHaveLength(2);
  });
});
