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
// ## Where a sign-in happens (#300, decided 2026-10-20)
//
// Always in the SYSTEM browser. It holds the user's GitHub session and every
// second factor GitHub offers; the in-app private window this used to open for
// connecting could not complete passkey, SMS or GitHub Mobile, so a passkey
// user could never connect at all.
//
// The hazard that window existed for is real: a browser already signed in to
// GitHub answers as THAT account. It is now caught where it can be caught for
// certain — the daemon's account guard refuses a callback from an account other
// than the one expected, or one that already belongs to another identity,
// before anything is stored. The list records which account a sign-in expects
// and its link, so the UI can say so and offer the link for a private window.

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
  /** The machine's default identity: the one owning the global git email (#296). */
  isDefault?: boolean;
}

export interface PersonaListDeps {
  /** Resolves `null` when the registry could not be read — never `[]`. */
  fetchPersonas: () => Promise<{ personas: PersonaWire[]; defaultError?: string | null } | null>;
  /** `POST /api/auth/signin`. `expectedLogin` is a hint GitHub uses to
   *  preselect an account; `null` when we have never learned one. */
  startSignIn: (persona: string, expectedLogin: string | null) => Promise<string | null>;
  /**
   * Open the daemon's authorize link in the SYSTEM browser — for every kind of
   * sign-in (#300, decided 2026-10-20). The in-app private window could not
   * complete passkey, SMS or GitHub Mobile. A browser signed in to GitHub as
   * someone else is caught by the daemon's account guard, and the UI offers the
   * link to copy into a private window.
   */
  openAuth: (url: string) => Promise<void>;
  /** `POST /api/auth/keychain/approve` (#283). Resolves `null` when approved,
   *  otherwise why not. */
  approveKeychain: (persona: string) => Promise<string | null>;
  /** `POST /api/auth/personas/{label}/default` — Make default. Resolves `null`
   *  when done, otherwise why not. */
  makeDefault: (label: string) => Promise<string | null>;
}

/** How often, and how many times, the list is re-read while a sign-in is open. */
const FOLLOW_MS = 3_000;
const FOLLOW_TICKS = 200;

export class PersonaList {
  personas = $state<PersonaWire[]>([]);
  /** Non-null when a read or an attempt FAILED. Rendered instead of a list:
   *  "no identities" and "we could not ask" are different answers. */
  error = $state<string | null>(null);

  /** Why no identity is the default, when none is (#296) — e.g. no global git
   *  email, or one that two identities own. `null` when one is, or unknown. */
  defaultError = $state<string | null>(null);

  /** The sign-in in flight, for the UI: which identity, which GitHub account it
   *  expects (`null` for a first connect), and the link to copy into a private
   *  window when the browser is signed in as someone else (#300). */
  pending = $state<{
    label: string;
    expectedLogin: string | null;
    url: string;
    /** The Keychain slot it signs into — the label becomes the verified login. */
    slot: string;
    /** The credential's expiry when the sign-in began: a new one means it finished. */
    was: number | null;
  } | null>(null);

  /** The re-read that follows a sign-in until it finishes. */
  #follow: ReturnType<typeof setInterval> | null = null;

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
    // `approve` too (#283): the session is there, but sync stays down until
    // the user lets the daemon read it.
    return this.personas.some((p) => p.action === 'signIn' || p.action === 'approve');
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
    this.defaultError = res.defaultError ?? null;
    this.error = null;
    this.loaded = true;
    this.#settle();
  }

  /** A sign-in has finished when its identity holds a NEW live credential; the
   *  notice then goes, and so does the re-read that waited for it. */
  #settle(): void {
    const pending = this.pending;
    if (!pending) return;
    const p = this.personas.find((x) => (x.sessionSlot ?? x.label) === pending.slot);
    if (!p || !p.connected || p.forgeToken.state !== 'active' || p.forgeToken.expiresAt === pending.was) return;
    this.pending = null;
    this.#stopFollowing();
  }

  #stopFollowing(): void {
    if (this.#follow) clearInterval(this.#follow);
    this.#follow = null;
  }

  /** Re-read every few seconds while a sign-in is open in the browser — for
   *  at most ten minutes, after which a person can re-open the list. */
  #followSignIn(): void {
    this.#stopFollowing();
    let left = FOLLOW_TICKS;
    this.#follow = setInterval(() => {
      if (!this.pending || --left < 0) return this.#stopFollowing();
      void this.load();
    }, FOLLOW_MS);
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
      case 'approve':
        return 'Approve keychain access';
      default:
        return 'Sign in';
    }
  }

  /** One line describing where this persona stands. */
  describe(p: PersonaWire, now: number): string {
    if (!p.connected) return 'never signed in on this machine';
    if (p.action === 'approve') return 'keychain access needs approval';
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
    if (p.action === 'approve') return 'warn';
    if (p.forgeToken.state === 'dead') return 'dead';
    const exp = p.forgeToken.expiresAt;
    if (exp === null) return 'idle';
    const seconds = exp - now;
    if (seconds <= 0) return 'dead';
    return seconds < 3600 ? 'warn' : 'ok';
  }

  /** The row's button (#283): approve the Keychain, or sign in. An approval
   *  opens no browser — the daemon re-reads the session with the Keychain's
   *  dialog allowed, and the user answers it there. */
  async act(p: PersonaWire): Promise<void> {
    if (p.action !== 'approve') return this.signIn(p);
    if (this.#busy.has(p.label)) return;
    this.#busy = new Set(this.#busy).add(p.label);
    this.error = null;
    try {
      this.error = await this.#deps
        .approveKeychain(p.sessionSlot ?? p.label)
        .catch((e) => (e instanceof Error ? e.message : `could not approve ${p.label}`));
    } finally {
      const next = new Set(this.#busy);
      next.delete(p.label);
      this.#busy = next;
    }
  }

  /** Make this identity the default (user, 2026-10-27), then re-read the list
   *  — the daemon decides which identity is the default, and says so. */
  async makeDefault(p: PersonaWire): Promise<void> {
    if (this.#busy.has(p.label)) return;
    this.#busy = new Set(this.#busy).add(p.label);
    try {
      this.error = await this.#deps
        .makeDefault(p.label)
        .catch((e) => (e instanceof Error ? e.message : `could not make ${p.label} the default`));
    } finally {
      const next = new Set(this.#busy);
      next.delete(p.label);
      this.#busy = next;
    }
    if (!this.error) await this.load();
  }

  /**
   * Start a sign-in for one persona, in the system browser (#300).
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
        this.pending = null;
        this.error = `could not start the sign-in for ${p.label}`;
        return;
      }
      const slot = p.sessionSlot ?? p.label;
      this.pending = { label: p.label, expectedLogin: p.githubLogin, url, slot, was: p.forgeToken.expiresAt };
      this.#followSignIn();
      await this.#deps.openAuth(url);
    } catch (e) {
      this.error = e instanceof Error ? e.message : `could not open your browser for ${p.label}`;
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
  approveKeychain: async (persona) => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryApproveKeychain(persona);
    if (!res.ok) return res.error.message || 'the daemon did not answer';
    return res.data.ok ? null : (res.data.error ?? 'keychain access was not approved');
  },
  makeDefault: async (label) => {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js')
    ]);
    const res = await senseiApi(appState.port).tryMakeDefaultPersona(label);
    return res.ok ? null : res.error.message || 'the daemon did not answer';
  },
  openAuth: async (url) => {
    // The system browser, for every sign-in (#300): it holds the user's GitHub
    // session and every second factor GitHub offers.
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  }
});
