// app/src/lib/forge-auth.svelte.ts
//
// The forge credential's standing, and the app's one job in the token
// lifecycle: re-authorize before the token dies.
//
// ## Why the app has to do this
//
// The GitHub token lives ~8 hours. Renewing it means redeeming a refresh token,
// which requires the App's client secret — and that secret stays in exactly one
// place, Supabase's auth provider config. Copying it into the daemon or the dōjō
// would mean recreating the App credential in two dashboards, with the missed
// copy failing silently months later.
//
// So renewal re-runs the authorize flow Supabase already performs: the same PKCE
// flow `POST /api/auth/signin` starts. For a user who has already authorized the
// App and still has a live GitHub session that is a redirect chain with no
// prompt — measured at ~6 seconds, zero clicks. But it needs a browser, which
// the daemon does not have. The app does.
//
// ## The rule that matters is about the SIDE EFFECT
//
// Renewal opens a browser. The renewal window is an hour wide and the standing
// only changes once the daemon has observed the new token, so a naive "renew
// while due" check firing once per poll would open a tab every minute at
// someone whose GitHub session had lapsed and who therefore never completed it.
//
// Hence [`ForgeAuth.#armed`]: ONE automatic attempt per window. If it does not
// resolve, the standing eventually becomes `signInNeeded`, and `SignInOverlay`
// (driven by `personas.svelte.ts`) offers the user a per-identity button — a
// choice, at a moment they picked.
//
// This store owns the AUTOMATIC half only. Everything the user clicks lives in
// `personas.svelte.ts`, because the subject there is a list of identities and
// this one reports a single persona's standing.

/** What the app believes about the forge credential. */
export type ForgeStanding =
  /** Not polled yet. Renders nothing — an absence of information, not good news. */
  | { kind: 'unknown' }
  | { kind: 'healthy'; expiresAt: number | null }
  /** Alive, but near enough to expiry to re-authorize now. */
  | { kind: 'renewalDue'; expiresAt: number | null }
  /** Dead or absent. Only the user can fix it. */
  | { kind: 'signInNeeded'; reason: string }
  /** We could not ask. Says NOTHING about the credential. */
  | { kind: 'unreachable'; detail: string }
  /** The daemon answered in a shape we do not understand. */
  | { kind: 'unreadable'; detail: string };

/**
 * Reduce a `/api/auth/status` body to a standing.
 *
 * Pure, and deliberately the same reasoning the CLI applies in `standing_of`:
 * trust the daemon's own `needsSignIn` and `renewalDue` flags rather than
 * re-deriving either. The margin lives in `forge_token_action`, once — a second
 * copy of "near expiry" drifts from the scheduler the moment either changes.
 */
export function standingOf(body: Record<string, unknown> | null): ForgeStanding {
  if (!body) return { kind: 'unreachable', detail: 'the daemon did not answer' };

  const needsSignIn = body.needsSignIn;
  if (body.signedIn !== true) {
    // NOT automatically signed out. The daemon separates a REJECTED session
    // (terminal — it already cleared the credential) from an UNREACHABLE dōjō
    // (transient — it deliberately kept it), and reports which via the same
    // `needsSignIn` flag. Reading only `signedIn` conflates them, which is how
    // a GoTrue 504 came to open a browser at a signed-in user.
    if (needsSignIn === true) return { kind: 'signInNeeded', reason: 'no usable session' };
    if (needsSignIn === false) {
      return {
        kind: 'unreachable',
        detail: typeof body.error === 'string' ? body.error : 'the dōjō could not be reached'
      };
    }
    // Absent: an older daemon. Refusing to act is the safe direction.
    return { kind: 'unreadable', detail: 'this daemon does not report whether a sign-in is needed' };
  }

  if (needsSignIn === true) return { kind: 'signInNeeded', reason: 'the forge token is dead' };

  const forge = (body.forgeToken ?? {}) as Record<string, unknown>;
  const expiresAt = typeof forge.expiresAt === 'number' ? forge.expiresAt : null;
  const state = forge.state;
  if (state === 'absent') return { kind: 'signInNeeded', reason: 'no forge token is stored' };
  if (state !== 'active' && state !== 'unknown') {
    return { kind: 'unreadable', detail: `unrecognised forge token state \`${String(state)}\`` };
  }
  if (body.renewalDue === true) return { kind: 'renewalDue', expiresAt };
  if (body.renewalDue === false) return { kind: 'healthy', expiresAt };
  // Missing the field entirely. Saying so beats assuming healthy, which would
  // make renewal silently never fire.
  return { kind: 'unreadable', detail: 'this daemon does not report `renewalDue` — reinstall it' };
}

/** The I/O this store needs, injected so every rule above is testable. */
export interface ForgeAuthDeps {
  /** `GET /api/auth/status`. Resolves `null` when the daemon could not be read —
   *  never a fabricated body. */
  fetchStatus: () => Promise<Record<string, unknown> | null>;
  /** `POST /api/auth/signin`, resolving to the authorize URL, or `null` if the
   *  daemon refused to start one. */
  startSignIn: () => Promise<string | null>;
  openUrl: (url: string) => Promise<void>;
}

export class ForgeAuth {
  standing = $state<ForgeStanding>({ kind: 'unknown' });
  /** The last failure worth showing. Cleared by a successful attempt. */
  lastError = $state<string | null>(null);
  /** Kept when the browser could not be opened, so the UI can offer the URL. */
  pendingUrl = $state<string | null>(null);

  /** Whether an automatic attempt is still available for THIS window.
   *  Re-armed when the standing leaves `renewalDue` — see the module note. */
  #armed = true;
  #timer: ReturnType<typeof setInterval> | null = null;
  #deps: ForgeAuthDeps;

  constructor(deps: ForgeAuthDeps) {
    this.#deps = deps;
  }

  /** Update from a status body. Pure — the poll and the tests share it. */
  apply(body: Record<string, unknown> | null): void {
    this.standing = standingOf(body);
    // Leaving the window re-arms the one automatic attempt. Without this the
    // app renews once per launch and never again.
    if (this.standing.kind !== 'renewalDue') this.#armed = true;
  }

  /** Poll the daemon, and re-authorize if that is now the right thing to do. */
  async poll(): Promise<void> {
    this.apply(await this.#deps.fetchStatus().catch(() => null));
    if (this.standing.kind === 'renewalDue' && this.#armed) {
      // Spent BEFORE the attempt, not after. Awaiting first would let a slow
      // authorize overlap the next poll and open a second tab.
      this.#armed = false;
      await this.#authorize();
    }
  }

  /**
   * Re-authorize now, whatever the standing.
   *
   * The banner's button. Ignores `#armed` deliberately: the user asked, and
   * refusing because we judged it unnecessary is how a button comes to feel
   * broken.
   */
  async renewNow(): Promise<void> {
    await this.#authorize();
  }

  async #authorize(): Promise<void> {
    this.lastError = null;
    this.pendingUrl = null;
    const url = await this.#deps.startSignIn().catch(() => null);
    if (!url) {
      this.lastError = 'could not start the authorize flow — is the dōjō reachable?';
      return;
    }
    try {
      await this.#deps.openUrl(url);
    } catch (e) {
      // Kept, so the UI can show the URL rather than leaving the user in front
      // of a button that appeared to do nothing.
      this.pendingUrl = url;
      this.lastError = e instanceof Error ? e.message : 'could not open a browser';
    }
  }

  /** Begin polling. Idempotent. Polls immediately, then every `intervalMs`. */
  start(intervalMs = 300_000): void {
    if (this.#timer) return;
    void this.poll();
    this.#timer = setInterval(() => {
      void this.poll();
    }, intervalMs);
  }

  stop(): void {
    if (this.#timer) clearInterval(this.#timer);
    this.#timer = null;
  }
}

/**
 * The app-wide instance, wired to the daemon and the Tauri opener.
 *
 * Imports are lazy for the same reason `daemon-health` defers its own: this
 * module must stay cheap to import from a unit test, and the Tauri plugin does
 * not exist in a browser dev session at all.
 */
export const forgeAuth = new ForgeAuth({
  fetchStatus: async () => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryGetAuthStatus();
    // A failed read stays a failed read. Returning `{}` here would land in the
    // `unreadable` arm and blame the daemon's shape for a network error.
    return res.ok ? res.data : null;
  },
  startSignIn: async () => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryStartSignIn();
    // A 200 is not proof of a URL: the daemon answers `{ok:false, error}` when
    // the dōjō refuses to build one. `?? null` keeps that a refusal rather than
    // handing `undefined` to the opener, which fails with nothing to show.
    return res.ok ? (res.data.authorizeUrl ?? null) : null;
  },
  openUrl: async (url) => {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  }
});
