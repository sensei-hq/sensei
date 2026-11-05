// Settings · Dōjō connection — how the identity-first screen reads (pure).
//
// Identities are the primary focus; once one is connected its dōjōs list, each
// with the repositories it has filed. The property under all of it: a skipped
// repository is a decision and a failed one is a fault — they never read alike.
import { describe, it, expect } from 'vitest';
import {
  TRAVELS,
  ownerOf,
  repoName,
  repoView,
  dojoGroups,
  identitySummary,
  isConnected,
  REPO_FILTERS,
  matchesFilter,
} from './dojo-connection.js';
import { identity, membership, repo } from './dojo-connection.fixtures.js';
import type { DojoRepositoryRow } from '$lib/types.js';

describe('what travels between sensei and the dōjō', () => {
  it('names the three directions, trackers read-only', () => {
    expect(TRAVELS.map((t) => t.direction)).toEqual(['out', 'both', 'in']);
    expect(TRAVELS[0].items).toContain('Metrics and diagrams');
    expect(TRAVELS[1].items).toContain('Teams and roles');
    expect(TRAVELS[2].items.join(' ')).toMatch(/GitHub Issues, Jira or Azure Boards/);
    expect(TRAVELS[2].note).toMatch(/read-only/);
  });
});

describe('repository keys', () => {
  it('owner is the forge owner, name drops the host', () => {
    expect(ownerOf('github.com/Sensei-HQ/sensei')).toBe('sensei-hq');
    expect(repoName('github.com/sensei-hq/sensei')).toBe('sensei-hq/sensei');
  });

  it('a key with no owner segment has no owner', () => {
    expect(ownerOf('local-only')).toBeNull();
    expect(repoName('local-only')).toBe('local-only');
  });
});

describe('repoView — how one repository reads', () => {
  it('synced: syncing, no notice, no date (the dōjō owns when)', () => {
    expect(repoView(repo())).toMatchObject({ name: 'sensei-hq/sensei', status: 'Syncing', category: 'syncing', notice: null });
  });

  it('pending: waiting for the next pass', () => {
    expect(repoView(repo({ state: 'pending', synced_at: null }))).toMatchObject({ status: 'Waiting for the next sync', category: 'syncing' });
  });

  it("skipped: a decision — a refusal Notice with the remedy's words, never a fault", () => {
    const v = repoView(repo({ state: 'skipped', last_error: 'not_subscribed', remedy: 'Private repositories sync on the Org plan.' }));
    expect(v.category).toBe('off');
    expect(v.status).toBe('Not syncing');
    expect(v.notice).toEqual({ tone: 'refusal', title: 'Private repositories sync on the Org plan.', code: 'not_subscribed' });
  });

  it('skipped with a known local reason: translated, not the raw code', () => {
    const v = repoView(repo({ state: 'skipped', last_error: 'index_not_current' }));
    expect(v.notice!.title).toMatch(/syncs once re-indexed/);
  });

  it("failing: a fault Notice with the failure's own text", () => {
    const v = repoView(repo({ state: 'error', last_error: 'the dojo refused: 402' }));
    expect(v.category).toBe('failing');
    expect(v.status).toBe('Failing');
    expect(v.notice).toEqual({ tone: 'fault', title: "This repository's last sync failed", detail: 'the dojo refused: 402', code: 'sync_error' });
  });

  it('partial: syncing, with what was left out', () => {
    const v = repoView(repo({ state: 'partial', last_error: '2 metrics were refused' }));
    expect(v.category).toBe('syncing');
    expect(v.notice).toEqual({ tone: 'normal', title: '2 metrics were refused', code: 'partial' });
  });

  it('an unknown state is shown, never dropped', () => {
    expect(repoView(repo({ state: 'weird' as never })).status).toBe('weird');
  });
});

describe('dojoGroups — the selected identity’s dōjōs and their repositories', () => {
  const personal = membership({ id: 'p', kind: 'personal', tenant_key: 'github/jerrythomas', org_slugs: ['jerrythomas'] });
  const org = membership({ id: 'o', kind: 'employer', tenant_key: 'github/sensei-hq', org_slugs: ['sensei-hq'] });
  const someoneElse = membership({ id: 'x', tenant_key: 'github/elsewhere', org_slugs: ['elsewhere'], persona_slot: 'jerry-seneca' });

  it("lists only this identity's memberships, each with the repositories its owners cover", () => {
    const rows = [
      repo({ entity_key: 'github.com/jerrythomas/burn-e' }),
      repo({ entity_key: 'github.com/sensei-hq/sensei' }),
      repo({ entity_key: 'github.com/elsewhere/x', persona: 'jerry-seneca' }),
    ];
    const g = dojoGroups([personal, org, someoneElse], rows, 'jerrythomas', 'all');
    expect(g.map((d) => d.id)).toEqual(['p', 'o']);
    expect(g[0].repos.map((r) => r.name)).toEqual(['jerrythomas/burn-e']);
    expect(g[1].repos.map((r) => r.name)).toEqual(['sensei-hq/sensei']);
  });

  it('names a dōjō by its tenant and kind, with the kind’s kanji', () => {
    const [p] = dojoGroups([personal], [], 'jerrythomas', 'all');
    expect(p).toMatchObject({ name: 'jerrythomas', kind: 'personal', kanji: '己', isOrg: false });
    const [o] = dojoGroups([org], [], 'jerrythomas', 'all');
    expect(o).toMatchObject({ name: 'sensei-hq', kind: 'employer', kanji: '社', isOrg: true });
  });

  it('counts syncing over all its repositories, whatever the filter shows', () => {
    const rows = [repo(), repo({ entity_key: 'github.com/sensei-hq/web', state: 'skipped', last_error: 'x' })];
    const [o] = dojoGroups([org], rows, 'jerrythomas', 'failing');
    expect(o.count).toBe('1 of 2 syncing');
    expect(o.repos).toEqual([]);
    expect(o.empty).toBe(true);
  });

  it("a repository no membership covers is filed under 'Not filed under a dōjō', never dropped", () => {
    const g = dojoGroups([org], [repo({ entity_key: 'github.com/stray/one' })], 'jerrythomas', 'all');
    expect(g.at(-1)).toMatchObject({ id: 'unfiled', name: 'Not filed under a dōjō' });
    expect(g.at(-1)!.repos.map((r) => r.name)).toEqual(['stray/one']);
  });

  it('only the repository verdict lists a repository — not its metric or graph rows', () => {
    const rows = [repo(), repo({ entity: 'repository_metric' }), repo({ entity: 'repository_graph', entity_key: 'github.com/sensei-hq/sensei@main' })];
    expect(dojoGroups([org], rows, 'jerrythomas', 'all')[0].repos).toHaveLength(1);
  });

  it('repositories sort failing first, then by name', () => {
    const rows = [
      repo({ entity_key: 'github.com/sensei-hq/b' }),
      repo({ entity_key: 'github.com/sensei-hq/c', state: 'error', last_error: 'x' }),
      repo({ entity_key: 'github.com/sensei-hq/a' }),
    ];
    expect(dojoGroups([org], rows, 'jerrythomas', 'all')[0].repos.map((r) => r.name)).toEqual(['sensei-hq/c', 'sensei-hq/a', 'sensei-hq/b']);
  });

  it('a repository written twice is one row — row keys stay unique', () => {
    const rows = [repo(), repo({ state: 'error', last_error: 'later' })];
    const [o] = dojoGroups([org], rows, 'jerrythomas', 'all');
    expect(o.repos).toHaveLength(1);
    expect(o.repos[0].category).toBe('failing');
  });

  it('a signed-out identity (no slot) has no dōjōs to show', () => {
    expect(dojoGroups([org], [repo()], null, 'all')).toEqual([]);
  });
});

// WP8 (D46; L0 ruling: the app toggle is L2's): a repository row carries the
// sync switch only where its dōjō says this person may change it.
describe('the sync switch on a repository row', () => {
  const personal = membership({ id: 'p', kind: 'personal', tenant_key: 'github/jerrythomas', org_slugs: ['jerrythomas'] });
  const org = membership({ id: 'o', kind: 'employer', tenant_key: 'github/sensei-hq', org_slugs: ['sensei-hq'] });
  const rows = [repo({ entity_key: 'github.com/jerrythomas/burn-e' }), repo({ entity_key: 'github.com/sensei-hq/sensei' })];
  const ruling = (key: string, configurable_by_me: boolean, elected = true) =>
    [key, { repo_key: key, elected, sync_enabled: elected, configurable_by_me }] as const;
  const toggles = (elections: Map<string, DojoRepositoryRow> | null) =>
    dojoGroups([personal, org], rows, 'jerrythomas', 'all', elections).flatMap((d) => d.repos.map((r) => [r.key, r.toggle]));

  it.skip('pending: wp8-daemon — shows where the person may change it, showing whether it is on', () => {
    const elections = new Map([
      ruling('github.com/jerrythomas/burn-e', true, false),
      ruling('github.com/sensei-hq/sensei', false),
    ]);
    expect(toggles(elections)).toEqual([
      ['github.com/jerrythomas/burn-e', { on: false }],
      ['github.com/sensei-hq/sensei', null],
    ]);
  });

  it('is absent where the dōjō has no ruling, or the rulings could not be read', () => {
    expect(toggles(new Map())).toEqual([
      ['github.com/jerrythomas/burn-e', null],
      ['github.com/sensei-hq/sensei', null],
    ]);
    expect(toggles(null).every(([, t]) => t === null)).toBe(true);
  });
});

describe('filters', () => {
  it('are all · syncing · not syncing · failing', () => {
    expect(REPO_FILTERS.map((f) => f.key)).toEqual(['all', 'syncing', 'off', 'failing']);
  });

  it.each([
    ['all', 'off', true],
    ['syncing', 'syncing', true],
    ['syncing', 'off', false],
    ['failing', 'failing', true],
  ] as const)('%s matches %s: %s', (filter, category, yes) => {
    expect(matchesFilter(filter, category)).toBe(yes);
  });
});

describe('identity cards', () => {
  it('connected: a summary of its dōjōs and what is syncing', () => {
    const ms = [membership({ id: 'a' }), membership({ id: 'b', kind: 'personal', org_slugs: ['jerrythomas'] })];
    expect(identitySummary(identity(), ms, [repo(), repo({ entity_key: 'github.com/sensei-hq/x', state: 'skipped' })])).toBe('2 dōjōs · 1 syncing');
  });

  it('one dōjō reads in the singular', () => {
    expect(identitySummary(identity(), [membership()], [])).toBe('1 dōjō · 0 syncing');
  });

  it('not connected: no summary', () => {
    expect(identitySummary(identity({ status: 'neverConnected', sessionSlot: null }), [membership()], [])).toBeNull();
  });

  it.each([
    ['ready', 'jerrythomas', true],
    ['expiring', 'jerrythomas', true],
    ['expired', 'jerrythomas', false],
    ['neverConnected', null, false],
    ['needsKeychain', 'jerrythomas', false],
  ] as const)('%s (slot %s) is connected: %s', (status, slot, yes) => {
    expect(isConnected(identity({ status, sessionSlot: slot }))).toBe(yes);
  });
});
