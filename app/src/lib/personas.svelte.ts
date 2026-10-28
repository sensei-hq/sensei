// app/src/lib/personas.svelte.ts
//
// The identity state behind the sign-in overlay AND Settings · Dōjō: every
// persona sensei knows, where each one stands, and the action that fixes it.
//
// ## One state decides; views render (user, 2026-10-28)
//
// The daemon reports FACTS per identity — connected, the token's state and
// expiry, whether renewal is due, what the Keychain said. This class decides
// everything from them and its own clock: the status, the sentence, the tone
// and the button. The overlay and the settings screen render `identities` and
// nothing else, so they cannot disagree. (They did: "credential expired" beside
// "ready", and a Keychain approval asked for a credential 14 hours dead.)
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

/** One persona's facts, exactly as `GET /api/auth/personas` reports them. */
export interface PersonaWire {
  label: string;
  githubLogin: string | null;
  /** The Keychain slot. `null` when never connected. NOT the label. */
  sessionSlot: string | null;
  connected: boolean;
  forgeToken: { state: string; expiresAt: number | null };
  /** Renewal is due, by the daemon's scheduler rule (its margin lives there). */
  renewalDue: boolean;
  /** What the Keychain said about the stored session; `null` when not read. */
  keychain: 'readable' | 'needsApproval' | 'missing' | null;
  /** The machine's default identity: the one owning the global git email (#296). */
  isDefault?: boolean;
}

export type IdentityStatus = 'neverConnected' | 'expired' | 'needsKeychain' | 'expiring' | 'ready';
export type IdentityTone = 'idle' | 'ok' | 'warn' | 'dead';
export interface IdentityAction {
  kind: 'connect' | 'signIn' | 'approve' | 'renew';
  label: string;
}

/** One identity as every view renders it — decided here, once. */
export interface Identity {
  label: string;
  githubLogin: string | null;
  sessionSlot: string | null;
  isDefault: boolean;
  status: IdentityStatus;
  expiresAt: number | null;
  /** The one sentence for where it stands. */
  standing: string;
  tone: IdentityTone;
  /** The button, or `null` when there is nothing to do. */
  action: IdentityAction | null;
  busy: boolean;
  /** The button's text, `opening…` while its sign-in starts; `null` with no button. */
  buttonLabel: string | null;
}

export interface PersonaListDeps {
  /** Resolves `null` when the registry could not be read — never `[]`. */
  fetchPersonas: () => Promise<{ personas: PersonaWire[]; defaultError?: string | null } | null>;
  /** `POST /api/auth/signin`. `expectedLogin` is a hint GitHub uses to
   *  preselect an account; `null` when we have never learned one. */
  startSignIn: (persona: string, expectedLogin: string | null) => Promise<string | null>;
  /** Open the daemon's authorize link in the SYSTEM browser (#300). */
  openAuth: (url: string) => Promise<void>;
  /** `POST /api/auth/keychain/approve` (#283). Resolves `null` when approved,
   *  otherwise why not. */
  approveKeychain: (persona: string) => Promise<string | null>;
  /** `POST /api/auth/personas/{label}/default`. `null` when done, else why not. */
  makeDefault: (label: string) => Promise<string | null>;
  /** Unix seconds. Injected so expiry can be tested. */
  now?: () => number;
}

/** Each tone's text colour — one map for every view that shows an identity. */
export const IDENTITY_TONE_CLASS: Record<IdentityTone, string> = {
  idle: 'text-ink-mute',
  ok: 'text-success',
  warn: 'text-warning',
  dead: 'text-danger'
};

/** How often, and how many times, the list is re-read while a sign-in is open. */
const FOLLOW_MS = 3_000;
const FOLLOW_TICKS = 200;
/** How often the clock moves and the facts are re-read while the app is open. */
const TICK_MS = 60_000;

const ACTIONS: Record<Exclude<IdentityStatus, 'ready'>, IdentityAction> = {
  // First-time. "Sign in again" would read as a fault the user caused.
  neverConnected: { kind: 'connect', label: 'Connect' },
  expired: { kind: 'signIn', label: 'Sign in again' },
  needsKeychain: { kind: 'approve', label: 'Approve keychain access' },
  expiring: { kind: 'renew', label: 'Renew' }
};

export class PersonaList {
  #wire = $state<PersonaWire[]>([]);
  /** The clock, in unix seconds. Moves on `tick`, so expiry is seen while open. */
  now = $state(0);
  /** Non-null when a read or an attempt FAILED. Rendered instead of a list:
   *  "no identities" and "we could not ask" are different answers. */
  error = $state<string | null>(null);
  /** Why no identity is the default, when none is (#296). */
  defaultError = $state<string | null>(null);
  /** The sign-in in flight, for the UI: which identity, which GitHub account it
   *  expects (`null` for a first connect), and the link to copy (#300). */
  pending = $state<{
    label: string;
    expectedLogin: string | null;
    url: string;
    /** The Keychain slot it signs into — the label becomes the verified login. */
    slot: string;
    /** The credential's expiry when the sign-in began: a new one means it finished. */
    was: number | null;
  } | null>(null);
  /** Whether a read has SUCCEEDED at least once — so nothing claims emptiness
   *  before the question is answered. */
  loaded = $state(false);

  /** Per-row, keyed by label, so one slow sign-in does not disable the list. */
  #busy = $state<Set<string>>(new Set());
  #follow: ReturnType<typeof setInterval> | null = null;
  #ticker: ReturnType<typeof setInterval> | null = null;
  #deps: PersonaListDeps;

  /** Called when an identity changed here — a sign-in finished, an approval,
   *  a new default — so the layout can tell the other windows, each of which
   *  holds its own copy of this state (as `appState.onConfigSaved`). */
  onChanged: (() => void) | null = null;

  constructor(deps: PersonaListDeps) {
    this.#deps = deps;
    this.now = this.#clock();
  }

  #clock(): number {
    return this.#deps.now ? this.#deps.now() : Math.floor(Date.now() / 1000);
  }

  /** Every identity, decided. The only thing views render. */
  identities = $derived(this.#wire.map((w) => this.#identity(w)));

  /** Whether something is BROKEN enough to open the overlay unasked: an
   *  expired credential, or a live one the Keychain withholds. Never-connected
   *  ones do not summon it (an install infers several), nor does a renewal
   *  (automatic and prompt-free). */
  get needsAttention(): boolean {
    return this.identities.some((i) => i.status === 'expired' || i.status === 'needsKeychain');
  }

  #status(w: PersonaWire): IdentityStatus {
    if (!w.connected) return 'neverConnected';
    const { state, expiresAt } = w.forgeToken;
    if (state === 'dead' || state === 'absent') return 'expired';
    // Half-open, as the daemon's scheduler: at the instant of expiry it is gone.
    if (expiresAt !== null && expiresAt <= this.now) return 'expired';
    // Only a credential still alive is worth approving the Keychain for.
    if (w.keychain === 'needsApproval') return 'needsKeychain';
    if (w.renewalDue) return 'expiring';
    return 'ready';
  }

  #identity(w: PersonaWire): Identity {
    const status = this.#status(w);
    const expiresAt = w.forgeToken.expiresAt;
    const left = expiresAt === null ? null : (expiresAt - this.now) / 3600;
    const standing =
      status === 'neverConnected' ? 'never signed in on this machine'
      : status === 'expired' ? 'credential expired'
      : status === 'needsKeychain' ? 'keychain access needs approval'
      : left === null ? 'signed in — expiry not yet known'
      : left < 1 ? `${Math.round(left * 60)} minutes left`
      : `${left.toFixed(1)} hours left`;
    // Calm with hours left, urgent within the hour; neutral when nothing is
    // known to be wrong (never connected, or an expiry not yet learned).
    const tone: IdentityTone =
      status === 'neverConnected' ? 'idle'
      : status === 'expired' ? 'dead'
      : status === 'needsKeychain' || status === 'expiring' ? 'warn'
      : left === null ? 'idle'
      : left < 1 ? 'warn'
      : 'ok';
    const action = status === 'ready' ? null : ACTIONS[status];
    const busy = this.#busy.has(w.label);
    return {
      label: w.label,
      githubLogin: w.githubLogin,
      sessionSlot: w.sessionSlot,
      isDefault: w.isDefault ?? false,
      status,
      expiresAt,
      standing,
      tone,
      action,
      busy,
      buttonLabel: action === null ? null : busy ? 'opening…' : action.label
    };
  }

  async load(): Promise<void> {
    const res = await this.#deps.fetchPersonas().catch(() => null);
    this.now = this.#clock();
    if (!res) {
      // Deliberately NOT an empty list: "you have no identities" invites a
      // connect against a registry that is merely unreachable.
      this.#wire = [];
      this.error = 'could not read the identity list — is the daemon running?';
      return;
    }
    this.#wire = res.personas;
    this.defaultError = res.defaultError ?? null;
    this.error = null;
    this.loaded = true;
    this.#settle();
  }

  /** Move the clock and re-read the facts. Runs every minute once started. */
  async tick(): Promise<void> {
    await this.load();
  }

  /** Keep the state current while the app is open: the clock moves, so a
   *  credential that expires on screen says so without anyone reloading. */
  start(): void {
    if (this.#ticker) return;
    this.#ticker = setInterval(() => void this.tick(), TICK_MS);
  }

  stop(): void {
    if (this.#ticker) clearInterval(this.#ticker);
    this.#ticker = null;
    this.#stopFollowing();
  }

  /** A sign-in has finished when its identity holds a NEW live credential; the
   *  notice then goes, the re-reads stop, and the other windows are told. */
  #settle(): void {
    const pending = this.pending;
    if (!pending) return;
    const w = this.#wire.find((x) => (x.sessionSlot ?? x.label) === pending.slot);
    if (!w || !w.connected || w.forgeToken.state !== 'active' || w.forgeToken.expiresAt === pending.was) return;
    this.pending = null;
    this.#stopFollowing();
    this.onChanged?.();
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

  #wireOf(id: Identity): PersonaWire | undefined {
    return this.#wire.find((w) => w.label === id.label);
  }

  async #withBusy(label: string, run: () => Promise<void>): Promise<void> {
    if (this.#busy.has(label)) return;
    this.#busy = new Set(this.#busy).add(label);
    try {
      await run();
    } finally {
      const next = new Set(this.#busy);
      next.delete(label);
      this.#busy = next;
    }
  }

  /** The row's button: approve the Keychain (#283) — no browser, the daemon
   *  re-reads with the Keychain's dialog allowed — or sign in. */
  async act(id: Identity): Promise<void> {
    if (id.action?.kind !== 'approve') return this.signIn(id);
    await this.#withBusy(id.label, async () => {
      this.error = await this.#deps
        .approveKeychain(id.sessionSlot ?? id.label)
        .catch((e) => (e instanceof Error ? e.message : `could not approve ${id.label}`));
    });
    // A refusal stays on screen; a re-read would clear it.
    if (this.error) return;
    await this.load();
    this.onChanged?.();
  }

  /** Make this identity the default, then re-read: the daemon decides which
   *  identity is the default, and says so. */
  async makeDefault(id: Identity): Promise<void> {
    await this.#withBusy(id.label, async () => {
      this.error = await this.#deps
        .makeDefault(id.label)
        .catch((e) => (e instanceof Error ? e.message : `could not make ${id.label} the default`));
    });
    if (this.error) return;
    await this.load();
    this.onChanged?.();
  }

  /**
   * Start a sign-in for one identity, in the system browser (#300). Addressed by
   * its Keychain slot when there is one: the label is rewritten to the verified
   * login on sign-in and the slot is not.
   */
  async signIn(id: Identity): Promise<void> {
    await this.#withBusy(id.label, async () => {
      this.error = null;
      try {
        const slot = id.sessionSlot ?? id.label;
        const url = await this.#deps.startSignIn(slot, id.githubLogin).catch(() => null);
        if (!url) {
          this.pending = null;
          this.error = `could not start the sign-in for ${id.label}`;
          return;
        }
        this.pending = { label: id.label, expectedLogin: id.githubLogin, url, slot, was: this.#wireOf(id)?.forgeToken.expiresAt ?? null };
        this.#followSignIn();
        await this.#deps.openAuth(url);
      } catch (e) {
        this.error = e instanceof Error ? e.message : `could not open your browser for ${id.label}`;
      }
    });
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
