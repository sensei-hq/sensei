// Settings · Dōjō connection — the screen's state (DojoConnectionScreen).
//
// One state decides; the components render (frontend guidelines §2). It owns
// which identity is selected, the filter, and every line the screen says; the
// identity facts come from PersonaList, the dōjōs and repositories from the
// page's load, and the actions go back to PersonaList.
import { describe, it, expect, vi } from 'vitest';
import { DojoConnectionScreen, type ConnectionSource } from './dojo-connection-state.svelte.js';
import { identity, membership, repo } from './dojo-connection.fixtures.js';

const ready = identity();
const expiring = identity({ label: 'jerry-seneca', name: 'jerry-seneca', githubLogin: 'jerry-seneca', sessionSlot: 'jerry-seneca', isDefault: false, status: 'expiring', standing: '24 minutes left', tone: 'warn', action: { kind: 'renew', label: 'Renew' }, buttonLabel: 'Renew', badge: null });
const fresh = identity({ label: 'jt-oss', name: 'jt-oss', githubLogin: null, sessionSlot: null, isDefault: false, status: 'neverConnected', standing: 'never signed in on this machine', tone: 'idle', action: { kind: 'connect', label: 'Connect' }, buttonLabel: 'Connect', badge: null });
const expired = identity({ label: 'old', name: 'old', sessionSlot: 'old', isDefault: false, status: 'expired', standing: 'credential expired', tone: 'dead', action: { kind: 'signIn', label: 'Sign in again' }, buttonLabel: 'Sign in again', badge: null });

function screen(over: Partial<ConnectionSource> = {}) {
  const src = $state<ConnectionSource>({
    identities: [ready, expiring, fresh],
    phase: 'list',
    identitiesError: null,
    memberships: [
      membership({ id: 'p', kind: 'personal', tenant_key: 'github/jerrythomas', org_slugs: ['jerrythomas'] }),
      membership({ id: 'o', tenant_key: 'github/sensei-hq', org_slugs: ['sensei-hq'] }),
      membership({ id: 's', tenant_key: 'github/senecaglobal', org_slugs: ['senecaglobal'], persona_slot: 'jerry-seneca' }),
    ],
    membershipsError: null,
    sync: [
      repo({ entity_key: 'github.com/jerrythomas/burn-e' }),
      repo({ entity_key: 'github.com/sensei-hq/sensei' }),
      repo({ entity_key: 'github.com/sensei-hq/web', state: 'skipped', last_error: 'not_subscribed', remedy: 'Private repositories sync on the Org plan.' }),
      repo({ entity_key: 'github.com/senecaglobal/handbook', persona: 'jerry-seneca' }),
    ],
    syncError: null,
    ...over,
  });
  const actions = { act: vi.fn(), makeDefault: vi.fn(), signOut: vi.fn(), reload: vi.fn() };
  return { s: new DojoConnectionScreen(() => src, actions), src, actions };
}

describe('which identity is selected', () => {
  it('the default identity until the reader picks one', () => {
    expect(screen().s.selected?.label).toBe('jerrythomas');
  });

  it('the first when none is the default', () => {
    const { s } = screen({ identities: [expiring, fresh] });
    expect(s.selected?.label).toBe('jerry-seneca');
  });

  it('the reader’s pick, and the default again if the pick disappears', () => {
    const { s, src } = screen();
    s.select('jt-oss');
    expect(s.selected?.label).toBe('jt-oss');
    src.identities = [ready, expiring];
    expect(s.selected?.label).toBe('jerrythomas');
  });

  it('none while there are no identities', () => {
    expect(screen({ identities: [], phase: 'empty' }).s.selected).toBeNull();
  });
});

describe('identity cards — status only; every action is in the header', () => {
  it('one card per identity, the selected one marked, with its summary', () => {
    const cards = screen().s.cards;
    expect(cards.map((c) => [c.label, c.selected])).toEqual([['jerrythomas', true], ['jerry-seneca', false], ['jt-oss', false]]);
    expect(cards[0]).toMatchObject({ name: 'jerrythomas', isDefault: true, standing: '6.5 hours left', toneClass: 'text-success', summary: '2 dōjōs · 2 syncing' });
    expect(cards[2].summary).toBeNull();
    expect(Object.keys(cards[0])).not.toContain('action');
  });
});

describe('the header — all of the selected identity’s actions, in one place', () => {
  it('connected and fine: Make default (when not) and Sign out; no primary', () => {
    const { s } = screen();
    s.select('jerry-seneca');
    s.select('jerrythomas');
    expect(s.header).toMatchObject({ name: 'jerrythomas', login: '@jerrythomas on GitHub', primary: null, canMakeDefault: false, canSignOut: true });
  });

  it('expiring: Renew is the primary; Make default and Sign out beside it', () => {
    const { s } = screen();
    s.select('jerry-seneca');
    expect(s.header).toMatchObject({ primary: { label: 'Renew' }, canMakeDefault: true, canSignOut: true, toneClass: 'text-warning' });
  });

  it('never connected: Connect with GitHub; nothing to sign out', () => {
    const { s } = screen();
    s.select('jt-oss');
    expect(s.header).toMatchObject({ login: 'Inferred from commit authorship', primary: { label: 'Connect with GitHub' }, canSignOut: false, canMakeDefault: false });
  });

  it('the buttons call PersonaList with the selected identity', () => {
    const { s, actions } = screen();
    s.select('jerry-seneca');
    s.runPrimary();
    s.runMakeDefault();
    s.runSignOut();
    expect(actions.act).toHaveBeenCalledWith(expiring);
    expect(actions.makeDefault).toHaveBeenCalledWith(expiring);
    expect(actions.signOut).toHaveBeenCalledWith(expiring);
  });

  it('a busy identity shows it on the primary', () => {
    const { s } = screen({ identities: [{ ...fresh, busy: true, buttonLabel: 'opening…' }] });
    expect(s.header?.primary).toEqual({ label: 'opening…', busy: true });
  });
});

describe('the body — connect, fix, or the dōjōs', () => {
  it('connected: the dōjōs', () => {
    expect(screen().s.body).toBe('dojos');
  });

  it('never connected: the connect steps', () => {
    const { s } = screen();
    s.select('jt-oss');
    expect(s.body).toBe('connect');
    expect(s.connectSteps[0]).toMatch(/Sign in as @jt-oss/);
  });

  it('expired: an action Notice saying what to do — its button is the header’s', () => {
    const { s } = screen({ identities: [expired] });
    expect(s.body).toBe('attention');
    expect(s.attention).toMatchObject({ tone: 'action', title: "old's credential expired" });
    expect(s.attention!.remedy).toMatch(/Sign in again/);
  });
});

describe('the dōjōs of the selected identity', () => {
  it('only its memberships, with their repositories', () => {
    const d = screen().s.dojos;
    expect(d.map((g) => g.name)).toEqual(['jerrythomas', 'sensei-hq']);
    expect(d[1].repos.map((r) => r.name)).toEqual(['sensei-hq/web', 'sensei-hq/sensei']);
  });

  it('a summary across them', () => {
    expect(screen().s.summary).toBe('2 dōjōs · 3 repositories · 2 syncing');
  });

  it('the filter narrows every dōjō, its counts stay whole', () => {
    const { s } = screen();
    s.setFilter('off');
    expect(s.dojos[1].repos.map((r) => r.name)).toEqual(['sensei-hq/web']);
    expect(s.dojos[1].count).toBe('1 of 2 syncing');
    expect(s.filters).toEqual([
      { key: 'all', label: 'All', count: 3, on: false },
      { key: 'syncing', label: 'Syncing', count: 2, on: false },
      { key: 'off', label: 'Not syncing', count: 1, on: true },
      { key: 'failing', label: 'Failing', count: 0, on: false },
    ]);
  });

  it('connected but in no dōjō: says so, as a normal Notice', () => {
    const { s } = screen({ memberships: [] , sync: [] });
    expect(s.dojos).toEqual([]);
    expect(s.noDojos).toMatchObject({ tone: 'normal', title: 'No dōjō lists @jerrythomas yet' });
  });

  it('a failed membership read is said, never shown as "no dōjō"', () => {
    const { s } = screen({ memberships: null, membershipsError: 'Failed to fetch' });
    expect(s.noDojos).toBeNull();
    expect(s.loadNotices).toEqual([{ tone: 'fault', title: "Couldn't load your dōjōs", detail: 'Failed to fetch', remedy: 'Check that the sensei daemon is running, then retry.', code: 'memberships' }]);
  });

  it('a failed sync read is said too, separately', () => {
    const { s } = screen({ sync: null, syncError: 'boom' });
    expect(s.loadNotices.map((n) => n.title)).toEqual(["Couldn't load what syncs"]);
  });

  // A failed read is never a list (H1, r1): nothing that depends on it draws a number.
  it('a failed membership read draws no dōjōs, no strays, no counts', () => {
    const { s } = screen({ memberships: null, membershipsError: 'Failed to fetch' });
    expect(s.dojos).toEqual([]);
    expect(s.summary).toBeNull();
    expect(s.filters).toEqual([]);
    expect(s.cards.map((c) => c.summary)).toEqual([null, null, null]);
  });

  it('a failed sync read names the dōjōs but draws no repositories or counts', () => {
    const { s } = screen({ sync: null, syncError: 'boom' });
    expect(s.dojos.map((d) => [d.id, d.count, d.repos.length, d.empty])).toEqual([
      ['p', null, 0, false],
      ['o', null, 0, false],
    ]);
    expect(s.summary).toBeNull();
    expect(s.filters).toEqual([]);
    expect(s.cards.map((c) => c.summary)).toEqual([null, null, null]);
  });
});

describe('identity errors', () => {
  it('a failed registry read: a fault Notice instead of the list', () => {
    const { s, actions } = screen({ identities: [], phase: 'error', identitiesError: 'registry unreachable' });
    const n = s.identitiesNotice!;
    expect(n).toMatchObject({ tone: 'fault', title: "Couldn't read your identities", detail: 'registry unreachable', code: 'personas',
      remedy: 'Check that the sensei daemon is running, then retry.' });
    // A failed registry read is not a dead end (H2): Retry reads it again.
    expect(n.primary).toMatchObject({ label: 'Retry', name: 'retry' });
    n.primary!.onclick!();
    expect(actions.reload).toHaveBeenCalledOnce();
  });

  it('an action that failed (a sign-in that would not open): said beside the list', () => {
    const { s } = screen({ identitiesError: 'could not open your browser for jt-oss' });
    expect(s.identitiesNotice).toEqual({ tone: 'fault', title: "That didn't go through", detail: 'could not open your browser for jt-oss', code: 'identity_action',
      remedy: 'Try it again from the identity above.' });
  });

  it('none when all is well', () => {
    expect(screen().s.identitiesNotice).toBeNull();
  });
});

describe("the identity's own sync — its plan and forge mirror", () => {
  const own = [
    repo({ entity: 'dojo_sync_plan', entity_key: 'dojo-a', direction: 'pull' }),
    repo({ entity: 'forge_mirror', entity_key: 'jerrythomas', direction: 'pull', state: 'error', last_error: 'could not reach', synced_at: null }),
  ];

  it('a failing one is a fault Notice in its own words', () => {
    const { s } = screen({ sync: own });
    expect(s.identityIssues).toEqual([{ key: 'forge_mirror:jerrythomas:pull', tone: 'fault', title: 'forge mirror is failing', detail: 'could not reach', code: 'forge_mirror',
      remedy: 'It retries on the next sync. If it keeps failing, sign this identity out and in again.' }]);
  });

  it('the activity lists each once, keyed so two dōjōs’ plans never collide', () => {
    const { s } = screen({ sync: [...own, repo({ entity: 'dojo_sync_plan', entity_key: 'dojo-b', direction: 'pull' })] });
    expect(s.activity.map((a) => a.key)).toEqual(['forge_mirror:jerrythomas:pull', 'dojo_sync_plan:dojo-a:pull', 'dojo_sync_plan:dojo-b:pull']);
    expect(s.activity[0]).toMatchObject({ label: 'forge mirror · pull', line: 'failing · never agreed' });
  });

  it('a tracker board’s push is the identity’s too, named by its key’s start, never a URL (tracker-delivery-metrics D3)', () => {
    const board = repo({ entity: 'tracker_board', entity_key: 'ab'.repeat(32), direction: 'push', state: 'error', last_error: 'dōjō returned 400: bad', synced_at: null });
    const { s } = screen({ sync: [board] });
    expect(s.activity).toMatchObject([{ label: 'board abababab · push', line: 'failing · never agreed' }]);
    expect(s.identityIssues).toMatchObject([{ title: 'board abababab is failing', detail: 'dōjō returned 400: bad' }]);
  });

  it("only the selected identity's", () => {
    const { s } = screen({ sync: [repo({ entity: 'dojo_sync_plan', entity_key: 'x', direction: 'pull', persona: 'jerry-seneca' })] });
    expect(s.activity).toEqual([]);
  });
});
