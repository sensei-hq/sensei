// app/src/lib/forge-auth.spec.svelte.ts
//
// The forge credential's standing in the app, and when the app is allowed to
// re-authorize on the user's behalf.
//
// The dangerous property here is not correctness of the reading — it is the
// SIDE EFFECT. Renewal opens a browser, so a rule that fires once per poll
// would open a tab every minute at someone whose GitHub session has lapsed.
// Most of these tests exist to pin that down.
import { describe, it, expect, vi } from 'vitest';
import { ForgeAuth, standingOf } from './forge-auth.svelte.js';

const NOW = 1_788_120_000;

/** The daemon's `/api/auth/status` shape, in each of its real forms. */
const payloads = {
  healthy: {
    signedIn: true,
    needsSignIn: false,
    renewalDue: false,
    forgeToken: { state: 'active', expiresAt: NOW + 25_200 }
  },
  due: {
    signedIn: true,
    needsSignIn: false,
    renewalDue: true,
    forgeToken: { state: 'active', expiresAt: NOW + 900 }
  },
  dead: {
    signedIn: true,
    needsSignIn: true,
    renewalDue: false,
    forgeToken: { state: 'dead', expiresAt: NOW - 60 }
  },
  signedOut: { signedIn: false, needsSignIn: true },
  // GoTrue returned 504; the daemon deliberately KEPT the stored session.
  outage: {
    signedIn: false,
    needsSignIn: false,
    error: 'could not reach dōjō — the stored session was left alone'
  },
  // A daemon predating `renewalDue`.
  older: { signedIn: true, forgeToken: { state: 'active', expiresAt: NOW + 900 } }
};

describe('standingOf', () => {
  it('reads the daemon rather than recomputing "near expiry"', () => {
    // The margin lives in `forge_token_action`, once. Recomputing it here from
    // the timestamp would drift from the scheduler the moment either changed.
    expect(standingOf(payloads.due)).toEqual({ kind: 'renewalDue', expiresAt: NOW + 900 });
    expect(standingOf(payloads.healthy)).toEqual({ kind: 'healthy', expiresAt: NOW + 25_200 });
  });

  it('does not mistake a dōjō outage for a signed-out user', () => {
    // Measured on the CLI before this existed: GoTrue answered 504, the daemon
    // said `signedIn: false` with the session left alone, and the caller
    // concluded the user was signed out — then opened a browser at them.
    expect(standingOf(payloads.outage).kind).toBe('unreachable');
  });

  it('treats a dead token and no session alike — both need the user', () => {
    expect(standingOf(payloads.dead).kind).toBe('signInNeeded');
    expect(standingOf(payloads.signedOut).kind).toBe('signInNeeded');
  });

  it('flags a daemon that omits renewalDue instead of assuming healthy', () => {
    // Defaulting to healthy would silently stop renewing on any daemon not yet
    // reinstalled — the exact failure this whole slice exists to remove.
    expect(standingOf(payloads.older).kind).toBe('unreadable');
  });
});

/** A ForgeAuth wired to canned status payloads and a spy opener. */
function harness(sequence: unknown[]) {
  let i = 0;
  const opened: string[] = [];
  const auth = new ForgeAuth({
    fetchStatus: vi.fn(async () => {
      const next = sequence[Math.min(i, sequence.length - 1)];
      i += 1;
      return next === null ? null : (next as Record<string, unknown>);
    }),
    startSignIn: vi.fn(async () => 'https://github.com/login/oauth/authorize?x=1'),
    openUrl: vi.fn(async (url: string) => {
      opened.push(url);
    })
  });
  return { auth, opened };
}

describe('ForgeAuth automatic renewal', () => {
  it('renews once when renewal falls due', async () => {
    const { auth, opened } = harness([payloads.due]);
    await auth.poll();
    expect(auth.standing.kind).toBe('renewalDue');
    expect(opened).toHaveLength(1);
  });

  it('does NOT open a second tab while the same window stays due', async () => {
    // The property that matters most. Renewal takes seconds but the standing
    // only changes once the daemon observes the new token, and the margin is an
    // hour wide — so a rule that fired per poll would open ~60 tabs at a user
    // whose GitHub session had lapsed and who therefore never completed it.
    const { auth, opened } = harness([payloads.due]);
    for (let n = 0; n < 20; n += 1) await auth.poll();
    expect(opened).toHaveLength(1);
  });

  it('arms again only after the standing leaves the renewal window', async () => {
    // Once renewal succeeds (healthy) the next window is a genuinely new event
    // and must be actionable, or the app renews once per launch and never again.
    const { auth, opened } = harness([payloads.due, payloads.healthy, payloads.due]);
    await auth.poll(); // due -> opens
    await auth.poll(); // healthy -> disarms
    await auth.poll(); // due again -> opens
    expect(opened).toHaveLength(2);
  });

  it('never opens a browser for a dead token or a signed-out user', async () => {
    // Both need the user, and neither is helped by a tab appearing unbidden at
    // launch. They get the sign-in overlay instead, with a button per identity —
    // the user chooses the moment, and which account.
    for (const p of [payloads.dead, payloads.signedOut]) {
      const { auth, opened } = harness([p]);
      await auth.poll();
      expect(auth.standing.kind).toBe('signInNeeded');
      expect(opened, JSON.stringify(p)).toHaveLength(0);
    }
  });

  it('never opens a browser when the dōjō is unreachable', async () => {
    const { auth, opened } = harness([payloads.outage]);
    await auth.poll();
    expect(opened).toHaveLength(0);
  });

  it('never opens a browser for a daemon it cannot read', async () => {
    const { auth, opened } = harness([payloads.older]);
    await auth.poll();
    expect(opened).toHaveLength(0);
  });

  it('reports the daemon being down without claiming a standing', async () => {
    // A failed read is not "healthy". Rendering nothing is fine; rendering
    // "your credential is fine" on the strength of an answer we never got is
    // the fabrication rule.
    const { auth, opened } = harness([null]);
    await auth.poll();
    expect(auth.standing.kind).toBe('unreachable');
    expect(opened).toHaveLength(0);
  });
});

describe('ForgeAuth.renewNow', () => {
  it('acts even when the app would not have acted on its own', async () => {
    // An explicit request. The user asked; second-guessing them because we
    // judged it unnecessary is how a button comes to feel broken.
    const { auth, opened } = harness([payloads.dead]);
    await auth.poll();
    expect(opened).toHaveLength(0);
    await auth.renewNow();
    expect(opened).toHaveLength(1);
  });

  it('surfaces a refusal instead of failing silently', async () => {
    const auth = new ForgeAuth({
      fetchStatus: async () => payloads.dead as Record<string, unknown>,
      // The daemon refuses when the dōjō will not build an authorize URL.
      startSignIn: async () => null,
      openUrl: async () => {
        throw new Error('should not be reached');
      }
    });
    await auth.poll();
    await auth.renewNow();
    expect(auth.lastError).toMatch(/could not start/i);
  });

  it('records an opener failure so the user is not left waiting', async () => {
    // A browser that never appears, with no message, looks exactly like a
    // button that does nothing.
    const auth = new ForgeAuth({
      fetchStatus: async () => payloads.due as Record<string, unknown>,
      startSignIn: async () => 'https://example.test/authorize',
      openUrl: async () => {
        throw new Error('no handler for https');
      }
    });
    await auth.poll();
    expect(auth.lastError).toMatch(/no handler/);
    // And the URL is kept so the UI can offer it as a copyable fallback.
    expect(auth.pendingUrl).toBe('https://example.test/authorize');
  });
});
