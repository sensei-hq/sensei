// app/src/lib/personas.spec.svelte.ts
//
// The identity list behind the sign-in overlay.
//
// Two properties carry the weight. First, each row's action comes from the
// DAEMON, never re-derived here. Second, the browsing context is chosen by what
// the action IS: renewal reuses the session (that is what makes it prompt-free),
// while connecting or re-authenticating demands a fresh one, because a shared
// session answers as whoever is already logged in and would re-link the wrong
// account while reporting success.
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
      label: 'jerrythomas',
      githubLogin: null,
      sessionSlot: null,
      connected: false,
      forgeToken: { state: 'unknown', expiresAt: null },
      action: 'connect'
    }
  ]
};

function harness(payload: unknown, url: string | null = 'https://github.test/authorize') {
  const opened: Array<{ url: string; fresh: boolean; label: string }> = [];
  const startSignIn = vi.fn(async () => url);
  const list = new PersonaList({
    fetchPersonas: vi.fn(async () => (payload === null ? null : (payload as never))),
    startSignIn,
    openAuth: vi.fn(async (u: string, opts: { fresh: boolean; label: string }) => {
      opened.push({ url: u, ...opts });
    })
  });
  return { list, opened, startSignIn };
}

describe('PersonaList.load', () => {
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
  });
});

describe('PersonaList.signIn', () => {
  it('CONNECTING a new identity opens a fresh context', async () => {
    // The system browser answers as whoever is already signed in to GitHub, so
    // connecting a second identity through it re-links the first — and reports
    // success, as the wrong person. A fresh context forces the real choice.
    const { list, opened } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[1]); // action: 'connect'
    expect(opened).toHaveLength(1);
    expect(opened[0].fresh).toBe(true);
  });

  it('RENEWING reuses the session, because that is what makes it silent', async () => {
    // The opposite case, and it looks inconsistent until you see why: renewal
    // is the SAME account, and the existing browser session is exactly what
    // lets GitHub redirect straight through with no prompt. A fresh context
    // here would turn a silent 6-second renewal into a login form every time.
    const { list, opened } = harness(LIST);
    await list.load();
    await list.signIn({ ...list.personas[0], action: 'renew' });
    expect(opened[0].fresh).toBe(false);
  });

  it('signing in again after a death also forces the account choice', async () => {
    // The credential is gone and we are re-authenticating. The login hint is
    // only a hint — GitHub may answer as a different signed-in account — so the
    // safe direction is to make the user pick.
    const { list, opened } = harness(LIST);
    await list.load();
    await list.signIn({ ...list.personas[0], action: 'signIn' });
    expect(opened[0].fresh).toBe(true);
  });

  it('gives each persona its own window so two sign-ins cannot share one', async () => {
    // A single reused label would have the second sign-in navigate the first
    // window, and whichever callback landed last would win.
    const { list, opened } = harness(LIST);
    await list.load();
    await list.signIn(list.personas[0]);
    await list.signIn(list.personas[1]);
    expect(new Set(opened.map((o) => o.label)).size).toBe(2);
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
    expect(startSignIn).toHaveBeenCalledWith('jerrythomas', null);
  });

  it('surfaces a refusal rather than opening nothing in silence', async () => {
    const { list, opened } = harness(LIST, null);
    await list.load();
    await list.signIn(list.personas[0]);
    expect(opened).toHaveLength(0);
    expect(list.error).toMatch(/could not start/i);
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
