// app/src/lib/personas.svelte.ts
//
// The identity list behind the sign-in overlay: every persona sensei knows,
// what each one needs, and the action that fixes it.
//
// ## Why a list and not one button
//
// sensei infers personas from commit authorship, so an install has several
// before anyone signs in — this machine has three. Each is a separate GitHub
// identity with its own credential and its own 8-hour expiry, so "sign in" is
// per-persona, not global.
//
// ## Which browsing context, and why it differs by action
//
// The two cases pull opposite ways, which looks inconsistent until you see the
// reason.
//
// **Renewing** an already-connected identity reuses the SYSTEM browser. Its live
// GitHub session is exactly what lets the authorize redirect complete with no
// prompt — measured at ~6 seconds, zero clicks. A fresh context here would turn
// every renewal into a login form.
//
// **Connecting** a new identity, or re-authenticating a dead one, opens an
// in-app `WebviewWindow` with `incognito: true` — a non-persistent data store,
// no inherited cookies. This is the hazard already documented on the daemon's
// `PersonaQuery.github_login`: a shared session answers as whoever is already
// signed in, so connecting a second persona through it re-links the FIRST one
// and reports success as the wrong person. The login hint only suggests an
// account; it cannot force one.
//
// Those windows are per-persona-labelled, so two concurrent sign-ins get
// separate windows instead of the second navigating the first.

/** One persona, exactly as `GET /api/auth/personas` reports it. */
export interface PersonaWire {
  label: string;
  githubLogin: string | null;
  /** The Keychain slot. `null` when never connected. NOT the label. */
  sessionSlot: string | null;
  connected: boolean;
  forgeToken: { state: string; expiresAt: number | null };
  /** Decided by the daemon's `persona_action`. Never re-derived here. */
  action: 'connect' | 'signIn' | 'renew' | 'none' | string;
}

export interface PersonaListDeps {
  /** Resolves `null` when the registry could not be read — never `[]`. */
  fetchPersonas: () => Promise<{ personas: PersonaWire[] } | null>;
  /** `POST /api/auth/signin`. `expectedLogin` is a hint GitHub uses to
   *  preselect an account; `null` when we have never learned one. */
  startSignIn: (persona: string, expectedLogin: string | null) => Promise<string | null>;
  /**
   * Show the authorize page.
   *
   * `fresh` decides WHICH browsing context, and the two cases pull opposite
   * ways:
   *
   * - **Renewing** the same account: reuse the session. The existing GitHub
   *   login is precisely what lets the redirect complete with no prompt —
   *   measured at ~6 seconds, no clicks. A fresh context would turn every
   *   renewal into a login form.
   * - **Connecting or re-authenticating**: a fresh context. A shared session
   *   answers as whoever is already logged in, so connecting a second identity
   *   silently re-links the first. The login hint is only a hint.
   */
  openAuth: (url: string, opts: { fresh: boolean; label: string }) => Promise<void>;
}

export class PersonaList {
  personas = $state<PersonaWire[]>([]);
  /** Non-null when a read or an attempt FAILED. Rendered instead of a list:
   *  "no identities" and "we could not ask" are different answers. */
  error = $state<string | null>(null);

  /**
   * Whether a read has SUCCEEDED at least once.
   *
   * Without this the overlay renders "No identities yet" the instant it opens —
   * an empty answer to a question not yet asked. Opening from the menu starts
   * the read and shows the overlay in the same tick, so the fabricated empty is
   * visible until the fetch lands, and stays visible if it fails.
   */
  loaded = $state(false);

  /** Per-row, keyed by label, so one slow sign-in does not disable the list. */
  #busy = $state<Set<string>>(new Set());
  #deps: PersonaListDeps;

  constructor(deps: PersonaListDeps) {
    this.#deps = deps;
  }

  isBusy(p: PersonaWire): boolean {
    return this.#busy.has(p.label);
  }

  /**
   * Whether something is BROKEN enough to open the overlay unasked.
   *
   * `signIn` only. Deliberately NOT `connect`: sensei infers personas from
   * commit authorship, so an install has several that have never been connected
   * and may never need to be — this machine has two. Counting those would pop
   * the overlay on every launch, which is the nuisance the automatic path exists
   * to avoid. They still appear IN the list; they just do not summon it.
   *
   * `renew` is excluded too, because renewal is automatic and prompt-free.
   */
  get needsAttention(): boolean {
    return this.personas.some((p) => p.action === 'signIn');
  }

  async load(): Promise<void> {
    const res = await this.#deps.fetchPersonas().catch(() => null);
    if (!res) {
      // Deliberately NOT an empty list. `[]` reads as "you have no identities",
      // and the action a user reaches for then is to connect one — the wrong
      // move against a registry that is merely unreachable.
      this.personas = [];
      this.error = 'could not read the identity list — is the daemon running?';
      return;
    }
    this.personas = res.personas;
    this.error = null;
    this.loaded = true;
  }

  /** The verb for this row's button. */
  actionLabel(p: PersonaWire): string {
    switch (p.action) {
      case 'connect':
        // First-time. "Sign in again" would read as a fault the user caused.
        return 'Connect';
      case 'signIn':
        return 'Sign in again';
      case 'renew':
        return 'Renew';
      default:
        return 'Sign in';
    }
  }

  /** One line describing where this persona stands. */
  describe(p: PersonaWire, now: number): string {
    if (!p.connected) return 'never signed in on this machine';
    const exp = p.forgeToken.expiresAt;
    if (p.forgeToken.state === 'dead') return 'credential expired';
    if (exp === null) return 'signed in — expiry not yet known';
    const hours = (exp - now) / 3600;
    if (hours <= 0) return 'credential expired';
    return hours < 1
      ? `${Math.round(hours * 60)} minutes left`
      : `${hours.toFixed(1)} hours left`;
  }

  /**
   * How alarming this persona's credential standing is.
   *
   * Beside [`describe`] because the two answer the same question in different
   * media — the sentence and the colour — and a second copy of the thresholds
   * would drift from it. `describe` switches wording at one hour; this switches
   * tone at the same boundary, from the same fields.
   *
   * The distinctions matter on a settings screen:
   *
   * * `idle` — never connected, or an expiry the daemon has not learned. Nothing
   *   is wrong. Colouring either red makes a fresh install look broken, and sends
   *   the user to re-authenticate a credential that is probably fine.
   * * `ok` — more than an hour left. Renewal is automatic and takes ~6s, so this
   *   needs no alarm.
   * * `warn` — under an hour. This is the state #127 exists for: the only current
   *   surfaces open AFTER death, so there is nowhere to see it coming.
   * * `dead` — expired. Distinct from `warn` because the action differs: one is a
   *   heads-up, the other is a thing you must now do.
   */
  tone(p: PersonaWire, now: number): 'idle' | 'ok' | 'warn' | 'dead' {
    if (!p.connected) return 'idle';
    if (p.forgeToken.state === 'dead') return 'dead';
    const exp = p.forgeToken.expiresAt;
    if (exp === null) return 'idle';
    const seconds = exp - now;
    if (seconds <= 0) return 'dead';
    return seconds < 3600 ? 'warn' : 'ok';
  }

  /**
   * Start a sign-in for one persona in its own fresh window.
   *
   * Addressed by `sessionSlot` when there is one. The label is rewritten to the
   * verified GitHub login on sign-in and the slot is not, so signing in against
   * the label addresses a persona that does not exist — the daemon then finds no
   * slot and silently connects nothing.
   */
  async signIn(p: PersonaWire): Promise<void> {
    if (this.#busy.has(p.label)) return;
    this.#busy = new Set(this.#busy).add(p.label);
    this.error = null;
    try {
      const url = await this.#deps
        .startSignIn(p.sessionSlot ?? p.label, p.githubLogin)
        .catch(() => null);
      if (!url) {
        this.error = `could not start the sign-in for ${p.label}`;
        return;
      }
      await this.#deps.openAuth(url, {
        // Renewal is the same account and wants the session; anything else
        // wants the user to choose. See `openAuth`.
        fresh: p.action !== 'renew',
        // Per-persona, so two concurrent sign-ins get separate windows instead
        // of the second navigating the first.
        label: `auth-${p.label.replace(/[^a-zA-Z0-9-]/g, '-')}`
      });
    } catch (e) {
      this.error = e instanceof Error ? e.message : `could not open a window for ${p.label}`;
    } finally {
      const next = new Set(this.#busy);
      next.delete(p.label);
      this.#busy = next;
    }
  }
}

/** The app-wide instance. Lazy imports, as `daemon-health` does. */
export const personaList = new PersonaList({
  fetchPersonas: async () => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryGetPersonas();
    return res.ok ? res.data : null;
  },
  startSignIn: async (persona, expectedLogin) => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryStartSignIn(persona, expectedLogin);
    // A 200 is not proof of a URL — the daemon answers `{ok:false, error}` when
    // the dōjō will not build one.
    return res.ok ? (res.data.authorizeUrl ?? null) : null;
  },
  openAuth: async (url, opts) => {
    if (!opts.fresh) {
      // The system browser, because it holds the GitHub session that makes a
      // renewal prompt-free. The app's own webview has never logged in, so an
      // in-app window would show a login form for a renewal that needs none.
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      return;
    }
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const w = new WebviewWindow(opts.label, {
      url,
      // A non-persistent data store: no inherited cookies, so GitHub asks
      // WHICH account instead of assuming the one already signed in.
      incognito: true,
      title: 'Sign in to GitHub',
      width: 520,
      height: 700,
      center: true
    });
    // A window that fails to construct must not resolve as success — the caller
    // records the failure and the UI offers the URL instead.
    await new Promise<void>((resolve, reject) => {
      void w.once('tauri://created', () => resolve());
      void w.once('tauri://error', (e) => reject(new Error(String(e.payload))));
    });
  }
});
