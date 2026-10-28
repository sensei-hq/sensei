// app/src/lib/personas.spec.svelte.ts
//
// The identity state behind the sign-in overlay AND Settings · Dōjō.
//
// The user (2026-10-28): "credential expired" beside "ready", a Keychain
// approval asked for a credential that had expired 14 hours earlier — because
// the daemon decided an `action`, the UI decided a sentence and a colour, and
// each screen combined them its own way. Now the daemon reports FACTS and this
// state decides everything — status, the sentence, the tone, the button — per
// identity, from those facts and its own clock. Views render what it says.
//
// Unchanged (#300, decided 2026-10-20): every sign-in opens the SYSTEM browser;
// the wrong account is caught by the daemon's guard, so the state records which
// account a sign-in expects and the link, for the UI to show and copy.
import { describe, it, expect, vi } from 'vitest';
import { PersonaList, type PersonaWire } from './personas.svelte.js';

const NOW = 1_788_200_000;
const H = 3600;

const live: PersonaWire = {
  label: 'sensei-hq-org',
  githubLogin: 'sensei-hq-org',
  sessionSlot: 'default',
  connected: true,
  forgeToken: { state: 'active', expiresAt: NOW + 7 * H },
  renewalDue: false,
  keychain: 'readable'
};
const fresh: PersonaWire = {
  label: 'devuser',
  githubLogin: null,
  sessionSlot: null,
  connected: false,
  forgeToken: { state: 'unknown', expiresAt: null },
  renewalDue: false,
  keychain: null
};
const LIST = { personas: [live, fresh] };

function harness(
  payload: unknown,
  url: string | null = 'https://github.test/authorize',
  approved: string | null = null,
  now = NOW
) {
  const opened: string[] = [];
  const startSignIn = vi.fn(async () => url);
  const approveKeychain = vi.fn(async () => approved);
  const announce = vi.fn();
  const clock = { now };
  const list = new PersonaList({
    fetchPersonas: vi.fn(async () => (payload === null ? null : (payload as never))),
    startSignIn,
    approveKeychain,
    openAuth: vi.fn(async (u: string) => {
      opened.push(u);
    }),
    makeDefault: vi.fn(async () => null),
    now: () => clock.now
  });
  list.onChanged = announce;
  return { list, opened, startSignIn, approveKeychain, announce, clock };
}

const one = async (p: Partial<PersonaWire>, now = NOW) => {
  const h = harness({ personas: [{ ...live, ...p }] }, undefined, null, now);
  await h.list.load();
  return h.list.identities[0];
};

describe('the status, decided once from the facts and the clock', () => {
  it('never connected is neutral and offers Connect, not a repair', async () => {
    const h = harness(LIST);
    await h.list.load();
    expect(h.list.identities[1]).toMatchObject({
      status: 'neverConnected',
      standing: 'never signed in on this machine',
      tone: 'idle',
      action: { kind: 'connect', label: 'Connect' }
    });
  });

  it('a dead or absent token is expired', async () => {
    for (const state of ['dead', 'absent']) {
      expect(await one({ forgeToken: { state, expiresAt: null } })).toMatchObject({
        status: 'expired',
        standing: 'credential expired',
        tone: 'dead',
        action: { kind: 'signIn', label: 'Sign in again' }
      });
    }
  });

  it('a live token past its expiry is expired — whatever else is reported', async () => {
    // The user's case: `active`, expired 14 hours ago, Keychain withholding it.
    // Expired wins: approving a dead credential only reveals that it is dead.
    const id = await one({ forgeToken: { state: 'active', expiresAt: NOW - 14 * H }, keychain: 'needsApproval' });
    expect(id).toMatchObject({ status: 'expired', standing: 'credential expired', action: { kind: 'signIn' } });
  });

  it('a withheld session on a live credential needs the Keychain approved', async () => {
    expect(await one({ keychain: 'needsApproval' })).toMatchObject({
      status: 'needsKeychain',
      standing: 'keychain access needs approval',
      tone: 'warn',
      action: { kind: 'approve', label: 'Approve keychain access' }
    });
  });

  it('renewal due is expiring, offered as Renew', async () => {
    expect(await one({ renewalDue: true, forgeToken: { state: 'active', expiresAt: NOW + 1800 } })).toMatchObject({
      status: 'expiring',
      tone: 'warn',
      action: { kind: 'renew', label: 'Renew' }
    });
  });

  it('ready says its remaining life and offers no button', async () => {
    const id = await one({});
    expect(id).toMatchObject({ status: 'ready', standing: '7.0 hours left', tone: 'ok', action: null });
    expect(await one({ forgeToken: { state: 'active', expiresAt: NOW + 1800 } })).toMatchObject({
      status: 'ready',
      standing: '30 minutes left',
      tone: 'warn'
    });
  });

  it('an unknown expiry is ready and neutral, never assumed expired', async () => {
    expect(await one({ forgeToken: { state: 'active', expiresAt: null } })).toMatchObject({
      status: 'ready',
      standing: 'signed in — expiry not yet known',
      tone: 'idle',
      action: null
    });
  });

  it('the clock moves the status: a credential that expires while open becomes expired', async () => {
    const h = harness({ personas: [{ ...live, forgeToken: { state: 'active', expiresAt: NOW + 60 } }] });
    await h.list.load();
    expect(h.list.identities[0].status).toBe('ready');
    h.clock.now = NOW + 61;
    await h.list.tick();
    expect(h.list.identities[0]).toMatchObject({ status: 'expired', standing: 'credential expired' });
  });
});

describe('PersonaList.needsAttention', () => {
  it('opens for an expired or withheld identity, not for one never connected or renewing', async () => {
    const cases: [Partial<PersonaWire>, boolean][] = [
      [{}, false],
      [{ forgeToken: { state: 'dead', expiresAt: null } }, true],
      [{ forgeToken: { state: 'active', expiresAt: NOW - 1 } }, true],
      [{ keychain: 'needsApproval' }, true],
      [{ renewalDue: true }, false]
    ];
    for (const [p, wanted] of cases) {
      const h = harness({ personas: [{ ...live, ...p }, fresh] });
      await h.list.load();
      expect(h.list.needsAttention, JSON.stringify(p)).toBe(wanted);
    }
  });
});

describe('PersonaList.load', () => {
  it('keeps which identity is the default, and why there is none when there is none (#296)', async () => {
    const { list } = harness({ personas: LIST.personas.map((p, i) => ({ ...p, isDefault: i === 0 })) });
    await list.load();
    expect(list.identities.map((p) => p.isDefault)).toEqual([true, false]);
    expect(list.defaultError).toBeNull();

    const unresolved = harness({ ...LIST, defaultError: 'no default identity: git has no global user.email' });
    await unresolved.list.load();
    expect(unresolved.list.defaultError).toMatch(/no global user.email/);
  });

  it('reports a failed read instead of showing an empty list', async () => {
    const { list } = harness(null);
    await list.load();
    expect(list.identities).toHaveLength(0);
    expect(list.error).toMatch(/could not/i);
    expect(list.loaded).toBe(false);
  });

  it('distinguishes a genuinely empty registry from a failure', async () => {
    const { list } = harness({ personas: [] });
    await list.load();
    expect(list.identities).toHaveLength(0);
    expect(list.error).toBeNull();
    expect(list.loaded).toBe(true);
  });

  it('is NOT loaded before the first read resolves', async () => {
    const { list } = harness(LIST);
    expect(list.loaded).toBe(false);
    await list.load();
    expect(list.loaded).toBe(true);
  });
});

describe('PersonaList.act', () => {
  it('approves the slot for a withheld session — no browser', async () => {
    const { list, startSignIn, approveKeychain, opened, announce } = harness({
      personas: [{ ...live, keychain: 'needsApproval' }]
    });
    await list.load();
    await list.act(list.identities[0]);
    expect(approveKeychain).toHaveBeenCalledWith('default');
    expect(startSignIn).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
    expect(announce).toHaveBeenCalled();
  });

  it('a declined approval says why', async () => {
    const { list } = harness({ personas: [{ ...live, keychain: 'needsApproval' }] }, null, 'declined');
    await list.load();
    await list.act(list.identities[0]);
    expect(list.error).toBe('declined');
  });

  it('every other button is a sign-in, in the system browser', async () => {
    const { list, opened, approveKeychain } = harness({
      personas: [{ ...live, forgeToken: { state: 'dead', expiresAt: null } }, fresh]
    });
    await list.load();
    await list.act(list.identities[0]);
    await list.act(list.identities[1]);
    expect(opened).toEqual(['https://github.test/authorize', 'https://github.test/authorize']);
    expect(approveKeychain).not.toHaveBeenCalled();
  });

  it('asks for the persona by its KEYCHAIN SLOT, the label only when there is none', async () => {
    const { list, startSignIn } = harness(LIST);
    await list.load();
    await list.signIn(list.identities[0]);
    await list.signIn(list.identities[1]);
    expect(startSignIn).toHaveBeenNthCalledWith(1, 'default', 'sensei-hq-org');
    expect(startSignIn).toHaveBeenNthCalledWith(2, 'devuser', null);
  });

  it('records the sign-in in flight: which identity, which account, and the link', async () => {
    const { list } = harness(LIST);
    await list.load();
    await list.signIn(list.identities[0]);
    expect(list.pending).toMatchObject({
      label: 'sensei-hq-org',
      expectedLogin: 'sensei-hq-org',
      url: 'https://github.test/authorize'
    });
  });

  it('surfaces a refusal rather than opening nothing in silence', async () => {
    const { list, opened } = harness(LIST, null);
    await list.load();
    await list.signIn(list.identities[0]);
    expect(opened).toHaveLength(0);
    expect(list.error).toMatch(/could not start/i);
    expect(list.pending).toBeNull();
  });

  it('marks only its own row busy, and the button says so', async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((r) => {
      release = r;
    });
    const slow = new PersonaList({
      fetchPersonas: async () => LIST as never,
      startSignIn: async () => {
        await gate;
        return 'https://github.test/a';
      },
      approveKeychain: async () => null,
      openAuth: async () => {},
      makeDefault: vi.fn(async () => null),
      now: () => NOW
    });
    await slow.load();
    const p = slow.signIn(slow.identities[1]);
    expect(slow.identities[1]).toMatchObject({ busy: true, buttonLabel: 'opening…' });
    expect(slow.identities[0].busy).toBe(false);
    release();
    await p;
    expect(slow.identities[1]).toMatchObject({ busy: false, buttonLabel: 'Connect' });
  });
});

describe('a sign-in in flight', () => {
  it('clears once the identity holds a NEW credential, and announces it', async () => {
    let payload: unknown = LIST;
    const announce = vi.fn();
    const list = new PersonaList({
      fetchPersonas: vi.fn(async () => payload as never),
      startSignIn: vi.fn(async () => 'https://github.test/authorize'),
      approveKeychain: vi.fn(async () => null),
      openAuth: vi.fn(async () => {}),
      makeDefault: vi.fn(async () => null),
      now: () => NOW
    });
    list.onChanged = announce;
    await list.load();
    await list.signIn(list.identities[0]);
    await list.load();
    expect(list.pending).not.toBeNull();

    const renewed = { ...live, label: 'renamed', forgeToken: { state: 'active', expiresAt: NOW + 8 * H } };
    payload = { personas: [renewed, fresh] };
    await list.load();
    expect(list.pending).toBeNull();
    expect(announce).toHaveBeenCalled();
  });

  it('is followed by re-reading the list until it finishes', async () => {
    vi.useFakeTimers();
    try {
      let payload: unknown = LIST;
      const fetchPersonas = vi.fn(async () => payload as never);
      const list = new PersonaList({
        fetchPersonas,
        startSignIn: vi.fn(async () => 'https://github.test/authorize'),
        approveKeychain: vi.fn(async () => null),
        openAuth: vi.fn(async () => {}),
        makeDefault: vi.fn(async () => null),
        now: () => NOW
      });
      await list.load();
      await list.signIn(list.identities[0]);
      const before = fetchPersonas.mock.calls.length;
      await vi.advanceTimersByTimeAsync(3_000);
      expect(fetchPersonas.mock.calls.length).toBeGreaterThan(before);
      payload = { personas: [{ ...live, forgeToken: { state: 'active', expiresAt: NOW + 8 * H } }] };
      await vi.advanceTimersByTimeAsync(3_000);
      expect(list.pending).toBeNull();
      list.stop();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('PersonaList.makeDefault', () => {
  it('asks the daemon, re-reads, and announces the change', async () => {
    const made: string[] = [];
    const fetchPersonas = vi.fn(async () => LIST as never);
    const announce = vi.fn();
    const list = new PersonaList({
      fetchPersonas,
      startSignIn: vi.fn(async () => null),
      approveKeychain: vi.fn(async () => null),
      openAuth: vi.fn(async () => {}),
      makeDefault: vi.fn(async (label: string) => {
        made.push(label);
        return null;
      }),
      now: () => NOW
    });
    list.onChanged = announce;
    await list.load();
    await list.makeDefault(list.identities[0]);
    expect(made).toEqual(['sensei-hq-org']);
    expect(fetchPersonas).toHaveBeenCalledTimes(2);
    expect(announce).toHaveBeenCalled();
    expect(list.error).toBeNull();
  });

  it('a refusal is said', async () => {
    const list = new PersonaList({
      fetchPersonas: vi.fn(async () => LIST as never),
      startSignIn: vi.fn(async () => null),
      approveKeychain: vi.fn(async () => null),
      openAuth: vi.fn(async () => {}),
      makeDefault: vi.fn(async () => 'no identity x'),
      now: () => NOW
    });
    await list.load();
    await list.makeDefault(list.identities[0]);
    expect(list.error).toBe('no identity x');
  });
});
