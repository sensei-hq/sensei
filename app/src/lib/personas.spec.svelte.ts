// app/src/lib/personas.spec.svelte.ts
//
// The identity list behind the sign-in overlay.
//
// Two properties carry the weight. First, each row's action comes from the
// DAEMON, never re-derived here. Second (#300, decided 2026-10-20): every
// sign-in opens the SYSTEM browser — the in-app window could not complete
// passkey, SMS or GitHub Mobile — and the wrong account is caught by the
// daemon's guard, not by a private window. So the list records which account a
// sign-in expects and the link, for the UI to show and to copy into a private
// window.
import { describe, it, expect, vi } from 'vitest';
import { PersonaList } from './personas.svelte.js';

const LIST = {
  personas: [
    {
      label: 'sensei-hq-org',
      githubLogin: 'sensei-hq-org',
      sessionSlot: 'default',
      connected: true,
      forgeToken: { state: 'active', expiresAt: 1_788_204_679 },
      action: 'none'
    },
    {
      label: 'devuser',
      githubLogin: null,
      sessionSlot: null,
      connected: false,
      forgeToken: { state: 'unknown', expiresAt: null },
      action: 'connect'
    }
  ]
};

function harness(
  payload: unknown,
  url: string | null = 'https://github.test/authorize',
  approved: string | null = null
) {
  const opened: string[] = [];
  const startSignIn = vi.fn(async () => url);
  const approveKeychain = vi.fn(async () => approved);
  const list = new PersonaList({
    fetchPersonas: vi.fn(async () => (payload === null ? null : (payload as never))),
    startSignIn,
    approveKeychain,
    openAuth: vi.fn(async (u: string) => {
      opened.push(u);
    })
  });
  return { list, opened, startSignIn, approveKeychain };
}

describe('PersonaList.load', () => {
  it('keeps which identity is the default, and why there is none when there is none (#296)', async () => {
    const { list } = harness({ ...LIST, personas: LIST.personas.map((p, i) => ({ ...p, isDefault: i === 0 })) });
    await list.load();
    expect(list.personas.map((p) => p.isDefault)).toEqual([true, false]);
    expect(list.defaultError).toBeNull();

    const unresolved = harness({ ...LIST, defaultError: 'no default identity: git has no global user.email' });
    await unresolved.list.load();
    expect(unresolved.list.defaultError).toMatch(/no global user.email/);
  });

  it('keeps the daemon-supplied action rather than deciding for itself', async () => {
    const { list } = harness(LIST);
    await list.load();
    expect(list.personas.map((p) => p.action)).toEqual(['none', 'connect']);
    expect(list.error).toBeNull();
  });

  it('reports a failed read instead of showing an empty list', async () => {
    // "You have no identities" invites the user to connect one, which is the
    // wrong action against a registry that is merely unreachable.
    const { list } = harness(null);
    await list.load();
    expect(list.personas).toHaveLength(0);
    expect(list.error).toMatch(/could not/i);
  });

  it('distinguishes a genuinely empty registry from a failure', async () => {
    const { list } = harness({ personas: [] });
    await list.load();
    expect(list.personas).toHaveLength(0);
    expect(list.error).toBeNull();
    expect(list.loaded).toBe(true);
  });

  it('is NOT loaded before the first read resolves', async () => {
    // Otherwise the overlay renders "No identities yet" over data we have not
    // fetched — an empty answer to a question we never asked. Visible for real:
    // opening from the menu sets the overlay open and starts the read in the
    // same tick, so the fabricated empty shows until the fetch lands.
    const { list } = harness(LIST);
    expect(list.loaded).toBe(false);
    await list.load();
    expect(list.loaded).toBe(true);
  });

  it('stays loaded=false when the read failed, so nothing claims emptiness', async () => {
    const { list } = harness(null);
    await list.load();
    expect(list.loaded).toBe(false);
    expect(list.error).toBeTruthy();
  });
});

describe('PersonaList.signIn', () => {
  it('opens the system browser with the daemon\'s link, for every kind of sign-in', async () => {
    const { list, opened } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[1]); // connect
    await list.signIn({ ...list.personas[0], action: 'renew' });
    await list.signIn({ ...list.personas[0], action: 'signIn' });
    expect(opened).toEqual([
      'https://github.test/authorize',
      'https://github.test/authorize',
      'https://github.test/authorize'
    ]);
  });

  it('records the sign-in in flight: which identity, which account, and the link', async () => {
    // What the UI shows ("sign in to GitHub as …") and what Copy link copies.
    const { list } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[0]);
    expect(list.pending).toMatchObject({
      label: 'sensei-hq-org',
      expectedLogin: 'sensei-hq-org',
      url: 'https://github.test/authorize'
    });
  });

  it('a finished sign-in clears the notice, once the identity holds a NEW credential', async () => {
    // The browser said "Signed in as …" while the app went on saying "Your
    // browser is open" and "credential expired" (2026-10-27): nothing re-read
    // the list or cleared the notice. A re-read that finds the same credential
    // (the sign-in still open) keeps it; a new one — matched by the Keychain
    // slot, since the label becomes the verified login — clears it.
    let payload: unknown = LIST;
    const list = new PersonaList({
      fetchPersonas: vi.fn(async () => payload as never),
      startSignIn: vi.fn(async () => 'https://github.test/authorize'),
      approveKeychain: vi.fn(async () => null),
      openAuth: vi.fn(async () => {})
    });
    await list.load();
    await list.signIn(list.personas[0]);
    await list.load();
    expect(list.pending).not.toBeNull();

    const fresh = { ...LIST.personas[0], label: 'renamed', forgeToken: { state: 'active', expiresAt: 1_900_000_000 } };
    payload = { personas: [fresh, LIST.personas[1]] };
    await list.load();
    expect(list.pending).toBeNull();
  });

  it('follows a sign-in by re-reading the list until it finishes', async () => {
    vi.useFakeTimers();
    try {
      let payload: unknown = LIST;
      const fetchPersonas = vi.fn(async () => payload as never);
      const list = new PersonaList({
        fetchPersonas,
        startSignIn: vi.fn(async () => 'https://github.test/authorize'),
        approveKeychain: vi.fn(async () => null),
        openAuth: vi.fn(async () => {})
      });
      await list.load();
      await list.signIn(list.personas[0]);
      const before = fetchPersonas.mock.calls.length;
      await vi.advanceTimersByTimeAsync(3_000);
      expect(fetchPersonas.mock.calls.length).toBeGreaterThan(before);
      payload = { personas: [{ ...LIST.personas[0], forgeToken: { state: 'active', expiresAt: 1_900_000_000 } }] };
      await vi.advanceTimersByTimeAsync(3_000);
      expect(list.pending).toBeNull();
      const settled = fetchPersonas.mock.calls.length;
      await vi.advanceTimersByTimeAsync(30_000);
      expect(fetchPersonas.mock.calls.length).toBe(settled);
    } finally {
      vi.useRealTimers();
    }
  });

  it('a first connect expects no particular account', async () => {
    const { list } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[1]);
    expect(list.pending).toMatchObject({ label: 'devuser', expectedLogin: null });
  });

  it('asks for the persona by its KEYCHAIN SLOT, not its display label', async () => {
    // The label is rewritten to the verified login on sign-in; the slot is not.
    // Signing in against the label addresses a persona that does not exist and
    // silently connects nothing.
    const { list, startSignIn } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[0]);
    expect(startSignIn).toHaveBeenCalledWith('default', 'sensei-hq-org');
  });

  it('falls back to the label when there is no slot yet', async () => {
    // A never-connected persona has no slot — the label is all there is, and it
    // is what the daemon will create the slot from.
    const { list, startSignIn } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[1]);
    expect(startSignIn).toHaveBeenCalledWith('devuser', null);
  });

  it('surfaces a refusal rather than opening nothing in silence', async () => {
    const { list, opened } = harness(LIST, null);
    await list.load();
    await list.signIn(list.personas[0]);
    expect(opened).toHaveLength(0);
    expect(list.error).toMatch(/could not start/i);
    expect(list.pending).toBeNull();
  });

  it('marks the row busy while its own sign-in is in flight, and no others', async () => {
    // Per-row, so one slow sign-in does not disable the whole list.
    const { list } = harness(LIST);
    await list.load();
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
      openAuth: async () => {}
    });
    await slow.load();
    const p = slow.signIn(slow.personas[0]);
    expect(slow.isBusy(slow.personas[0])).toBe(true);
    expect(slow.isBusy(slow.personas[1])).toBe(false);
    release();
    await p;
    expect(slow.isBusy(slow.personas[0])).toBe(false);
  });
});

describe('PersonaList.needsAttention', () => {
  it('does NOT open for identities that were merely never connected', async () => {
    // sensei infers personas from commit authorship, so this machine has two
    // that have never been connected and may never need to be. Treating those
    // as "attention needed" pops the overlay on every single launch — the exact
    // nuisance the automatic path exists to avoid.
    const { list } = harness({
      personas: [
        LIST.personas[0], // action: none
        LIST.personas[1] // action: connect
      ]
    });
    await list.load();
    expect(list.needsAttention).toBe(false);
  });

  it('opens when a CONNECTED identity has broken', async () => {
    // This one is different in kind: it worked, it stopped, and sync is silently
    // down until the user acts.
    const { list } = harness({
      personas: [{ ...LIST.personas[0], action: 'signIn' }, LIST.personas[1]]
    });
    await list.load();
    expect(list.needsAttention).toBe(true);
  });

  it('does not open merely because renewal is due', async () => {
    // Renewal is automatic and prompt-free. Announcing it would be noise.
    const { list } = harness({ personas: [{ ...LIST.personas[0], action: 'renew' }] });
    await list.load();
    expect(list.needsAttention).toBe(false);
  });
});

describe('PersonaList presentation', () => {
  it('labels the action by what it actually is', async () => {
    // "Sign in" for an identity that was never connected reads as a fault the
    // user caused; "Connect" is the truthful first-time verb.
    const { list } = harness(LIST);
    await list.load();
    expect(list.actionLabel(list.personas[1])).toBe('Connect');
    expect(list.actionLabel({ ...list.personas[0], action: 'signIn' })).toBe('Sign in again');
    expect(list.actionLabel({ ...list.personas[0], action: 'renew' })).toBe('Renew');
    expect(list.actionLabel(list.personas[0])).toBe('Sign in');
  });

  it('describes a connected persona by its remaining life, never a negative', async () => {
    const { list } = harness(LIST);
    await list.load();
    const text = list.describe(list.personas[0], 1_788_204_679 - 25_200);
    expect(text).toMatch(/7\.0 hours/);
    const past = list.describe(list.personas[0], 1_788_204_679 + 9999);
    expect(past).not.toContain('-');
  });

  it('says an unconnected persona has never signed in rather than showing no expiry', async () => {
    const { list } = harness(LIST);
    await list.load();
    expect(list.describe(list.personas[1], 0)).toMatch(/never/i);
  });
});

describe('PersonaList.tone', () => {
  // The whole point of #127: renewal is automatic and takes ~6s, so a credential
  // with hours left needs no alarm — but when it CANNOT renew there is currently
  // nowhere to see it coming. The tone is what makes "30 minutes left" look
  // different from "7 hours left" at a glance.
  it('is calm with hours left and urgent within the hour', async () => {
    const { list } = harness(LIST);
    await list.load();
    const p = list.personas[0];
    const exp = 1_788_204_679;
    expect(list.tone(p, exp - 25_200)).toBe('ok'); // 7 hours
    expect(list.tone(p, exp - 1_800)).toBe('warn'); // 30 minutes
  });

  it('is urgent for an expired credential, not merely a warning', async () => {
    const { list } = harness(LIST);
    await list.load();
    const dead = { ...list.personas[0], forgeToken: { state: 'dead', expiresAt: 1 } };
    expect(list.tone(dead, 2)).toBe('dead');
  });

  it('treats a never-connected persona as neutral, not as a failure', async () => {
    // Nothing is wrong with an identity you have not connected. Colouring it red
    // makes the settings screen look broken on a fresh install.
    const { list } = harness(LIST);
    await list.load();
    expect(list.tone(list.personas[1], 0)).toBe('idle');
  });

  it('is neutral when the expiry is unknown rather than assuming the worst', async () => {
    // `expiresAt` is null for a session whose expiry the daemon has not learned.
    // Rendering that as expired would send the user to re-authenticate a
    // credential that is probably fine.
    const { list } = harness(LIST);
    await list.load();
    const unknown = { ...list.personas[0], forgeToken: { state: 'active', expiresAt: null } };
    expect(list.tone(unknown, 0)).toBe('idle');
  });
});

// #283: the daemon never prompts for the Keychain, so a session it holds but
// cannot read after a rebuild waits for the user's click. That click is an
// approval, not a sign-in: no browser, no OAuth round trip.
describe('PersonaList approve', () => {
  // A live credential the Keychain withholds: approving is all it needs.
  const withheld = { ...LIST.personas[0], action: 'approve', forgeToken: { state: 'active', expiresAt: 4_000_000_000 } };

  it('opens the overlay unasked: sync is down until it is done', async () => {
    const { list } = harness({ personas: [withheld] });
    await list.load();
    expect(list.needsAttention).toBe(true);
  });

  it('says what it is and what the button does', async () => {
    const { list } = harness({ personas: [withheld] });
    await list.load();
    const p = list.personas[0];
    expect(list.actionLabel(p)).toBe('Approve keychain access');
    expect(list.describe(p, 0)).toBe('keychain access needs approval');
    expect(list.tone(p, 0)).toBe('warn');
  });

  it('approves the slot instead of starting a sign-in', async () => {
    const { list, startSignIn, approveKeychain, opened } = harness({ personas: [withheld] });
    await list.load();
    await list.act(list.personas[0]);
    expect(approveKeychain).toHaveBeenCalledWith('default');
    expect(startSignIn).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
    expect(list.error).toBeNull();
  });

  it('a declined or failed approval says why', async () => {
    const { list } = harness({ personas: [withheld] }, null, 'keychain access needs approval');
    await list.load();
    await list.act(list.personas[0]);
    expect(list.error).toBe('keychain access needs approval');
  });

  // 2026-10-27: approving a dead credential only revealed that it was dead —
  // a second click for a fact the daemon already knew. The sign-in has to
  // replace the withheld item, so the approval stays; the second click goes.
  const expired = { ...withheld, forgeToken: { state: 'dead', expiresAt: 1_700_000_000 } };

  it('a withheld credential that has expired says both, up front', async () => {
    const { list } = harness({ personas: [expired] });
    await list.load();
    const p = list.personas[0];
    expect(list.describe(p, 1_788_000_000)).toBe('credential expired — approve keychain access, then sign in');
    expect(list.actionLabel(p)).toBe('Approve and sign in');
    expect(list.tone(p, 1_788_000_000)).toBe('dead');
  });

  it('approves, then goes straight into the sign-in', async () => {
    const { list, startSignIn, approveKeychain, opened } = harness({ personas: [expired] });
    await list.load();
    await list.act(list.personas[0]);
    expect(approveKeychain).toHaveBeenCalledWith('default');
    expect(startSignIn).toHaveBeenCalledWith('default', 'sensei-hq-org');
    expect(opened).toEqual(['https://github.test/authorize']);
  });

  it('a declined approval does not start the sign-in', async () => {
    const { list, startSignIn } = harness({ personas: [expired] }, 'https://github.test/authorize', 'declined');
    await list.load();
    await list.act(list.personas[0]);
    expect(startSignIn).not.toHaveBeenCalled();
    expect(list.error).toBe('declined');
  });

  it('any other action is still a sign-in', async () => {
    const { list, startSignIn, approveKeychain } = harness({
      personas: [{ ...LIST.personas[0], action: 'signIn' }]
    });
    await list.load();
    await list.act(list.personas[0]);
    expect(startSignIn).toHaveBeenCalled();
    expect(approveKeychain).not.toHaveBeenCalled();
  });
});
