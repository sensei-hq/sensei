// @vitest-environment jsdom
// Settings · Dōjō (user, 2026-10-27): the sync list grouped identity → day →
// repository, each identity by its NAME, tagged default or offered Make default.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { personaList } from '$lib/personas.svelte.js';
import Page from './+page.svelte';
import type { SyncStateRow } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const json = (body: unknown) => ({ ok: true, status: 200, statusText: 'OK', json: async () => body });
const row = (over: Partial<SyncStateRow>): SyncStateRow => ({
  entity: 'repository_metric', entity_key: 'github.com/sensei-hq/torii', direction: 'push', state: 'synced',
  last_error: null, attempted_at: '2026-10-27T10:00:00Z', synced_at: '2026-10-27T10:00:00Z',
  updated_at: '2026-10-27T10:00:00Z', persona: 'jerrythomas', ...over,
});
const personas = {
  default: 'jerrythomas',
  personas: [
    { label: 'jerrythomas', githubLogin: 'jerrythomas', sessionSlot: 'jerrythomas', connected: true, action: 'none', isDefault: true, forgeToken: { state: 'active', expiresAt: 4_000_000_000 } },
    { label: 'sensei-hq-org', githubLogin: 'sensei-hq-org', sessionSlot: 'default', connected: true, action: 'signIn', isDefault: false, forgeToken: { state: 'dead', expiresAt: 1_700_000_000 } },
  ],
};
const entities = [
  row({}),
  row({ entity: 'repository_graph', entity_key: 'github.com/sensei-hq/torii@develop', state: 'skipped', synced_at: null, last_error: 'not_a_tracked_branch' }),
  row({ entity: 'forge_mirror', entity_key: 'default', direction: 'pull', state: 'error', synced_at: null, last_error: 'could not reach', persona: 'default' }),
];
const sync = { count: entities.length, counts: { synced: 1, skipped: 1, error: 1 }, entities };

describe('Settings · Dōjō', () => {
  it('groups each identity’s sync under its name, folds branches into the repository, and offers Make default', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string) => json(url.includes('/api/auth/personas') ? personas : {})));
    await personaList.load();
    const m = mountComponent(Page, { data: { sync, error: null } });
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-identity]').length).toBe(2));
    const cards = [...m.container.querySelectorAll('[data-identity]')] as HTMLElement[];
    expect(cards.map((c) => c.dataset.identity)).toEqual(['jerrythomas', 'sensei-hq-org']);
    // The slot name `default` is no identity's heading.
    expect(cards.some((c) => c.querySelector('[data-identity-name]')?.textContent?.trim() === 'default')).toBe(false);
    expect(cards[0].querySelector('[data-default-persona]')).not.toBeNull();
    expect(cards[1].querySelector('[data-make-default]')).not.toBeNull();
    // torii and torii@develop are one repository, the branch on its line.
    const repos = [...cards[0].querySelectorAll('[data-repository]')].map((r) => (r as HTMLElement).dataset.repository);
    expect(repos).toEqual(['github.com/sensei-hq/torii']);
    expect(cards[0].querySelector('[data-branch="develop"]')).not.toBeNull();
    // The forge mirror is sensei-hq-org's own, under its card.
    expect(cards[1].querySelector('[data-entity="forge_mirror"]')).not.toBeNull();
    // One sign-in notice for the screen, not one per identity.
    expect(m.container.querySelectorAll('[data-sign-in-pending]').length).toBeLessThanOrEqual(1);
  });
});
