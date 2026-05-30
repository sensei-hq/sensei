import type {
  HealthPayload, HealthEvent, Component, HealthStatus, Platform,
  PackageManagerId, ComponentId, Remedy,
} from './health-types.js';
import { COMPONENT_ORDER } from './health-types.js';
import { RealTransport, type HealthTransport } from './health-transport.js';
import { setHealthReady, clearHealthCache } from './health-cache.js';

/** Display labels for each ledger component. The Rust crate provides these in Phase 2/3 — here they live as cold-load defaults so the UI matches the mockup before any transport runs. */
const COMPONENT_DEFAULTS: Record<ComponentId, { label: string; note: string | null; installingVerb: string }> = {
  postgres: { label: 'PostgreSQL',         note: null,                            installingVerb: 'starting' },
  ollama:   { label: 'Ollama',             note: null,                            installingVerb: 'starting' },
  sensei:   { label: 'Sensei components',  note: 'cli · mcp · daemon',            installingVerb: 'installing' },
  database: { label: 'Database & schema',  note: 'pgvector · sensei tables',      installingVerb: 'setting up' },
  daemon:   { label: 'Background daemon',  note: null,                            installingVerb: 'starting' },
};

/** Poetic one-liner per gate, shown beneath the label in the ledger. Frontend
 *  owns this copy — it never travels on the wire. apply() and #patch() rewrite
 *  the field on every state mutation so wire payloads that carry a stale or
 *  missing value can't pollute the UI. */
const DESCRIPTIONS: Record<ComponentId | PackageManagerId, string> = {
  homebrew: 'The gardener who tends the tools.',
  winget:   'The gardener who tends the tools.',
  postgres: 'A still pond where memories settle.',
  ollama:   'A mind that thinks without leaving the room.',
  sensei:   'Three hands of the practice — speak, listen, attend.',
  database: 'Shelves shaped to the form of each memory.',
  daemon:   'The quiet breath that keeps watch.',
};

function emptyComponent(id: ComponentId): Component {
  const d = COMPONENT_DEFAULTS[id];
  // status='checking' (not 'pending') from the start. The Rust probe pipeline
  // begins the moment init() fires; if components defaulted to 'pending', the
  // ledger renders at 55% opacity with idle gray dots and the literal text
  // "pending" — visually indistinguishable from a frozen page — until the
  // first probe lands ~5s later. 'checking' lights up the busy state
  // immediately so the page reads as actively working.
  return {
    id, label: d.label, note: d.note, status: 'checking', version: null, detail: null,
    installingVerb: d.installingVerb, description: DESCRIPTIONS[id],
  };
}

/** Deterministic default — status='checking' so the UI never flashes 'ok' pre-apply. */
export const emptyPayload: HealthPayload = {
  version: '',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: {
    id: 'homebrew', label: 'Homebrew', note: 'which brew', status: 'checking',
    version: null, detail: null, installingVerb: 'installing',
    description: DESCRIPTIONS.homebrew,
  },
  components: COMPONENT_ORDER.map(emptyComponent),
  status: 'checking',
  remedy: null,
};

export class HealthState {
  status         = $state<HealthStatus>('checking');
  version        = $state<string>('');
  latest         = $state<string | null>(null);
  platform       = $state<Platform>('macos');
  packageManager = $state<Component>(emptyPayload.packageManager);
  components     = $state<Component[]>(emptyPayload.components);
  remedy         = $state<Remedy | null>(null);

  #transport: HealthTransport;
  #initPromise: Promise<void> | null = null;
  #verifyPromise: Promise<void> | null = null;

  get isOk():        boolean { return this.status === 'ok'; }
  get isBusy():      boolean { return this.status === 'checking' || this.status === 'resolving'; }
  get needsAction(): boolean { return this.status === 'needs-action'; }

  constructor(
    seed: HealthPayload = emptyPayload,
    transport: HealthTransport = new RealTransport(),
  ) {
    this.#transport = transport;
    this.apply(seed);
  }

  /** Force a fresh check. Clears the session cache. Same idempotency while in flight. */
  verify(): Promise<void> {
    if (this.#verifyPromise) return this.#verifyPromise;
    clearHealthCache();
    this.#initPromise = null;
    this.#verifyPromise = this.#runCheckThenMaybeResolve().then(() => {
      this.#verifyPromise = null;
    });
    this.#initPromise = this.#verifyPromise;
    return this.#verifyPromise;
  }

  /** Idempotent — runs the check once per app load. Concurrent callers share one in-flight promise. */
  async init(): Promise<void> {
    if (this.#initPromise) return this.#initPromise;
    this.#initPromise = this.#runCheckThenMaybeResolve();
    return this.#initPromise;
  }

  async #runCheckThenMaybeResolve(): Promise<void> {
    this.status = 'checking';
    // Use the streaming path for the whole check-and-resolve flow.
    // `health_check_and_resolve` (Rust side) emits a Component event after
    // each probe finishes, so the UI updates incrementally instead of
    // waiting 5-13s for the entire check phase to complete and arrive as
    // one atomic payload. The Rust pipeline still runs check → report →
    // resolve internally; we just consume it via the streaming listener.
    await this.#transport.resolve(emptyPayload, (ev) => this.applyEvent(ev));
  }

  apply(p: HealthPayload): void {
    // INV-1: discriminated union covers compile-time; this is the runtime boundary
    if (p.status === 'needs-action' && p.remedy === null)
      throw new Error('HealthState: needs-action requires a remedy');
    const pAsUntyped = p as unknown as { status: string; remedy: Remedy | null };
    if (pAsUntyped.remedy !== null && pAsUntyped.status !== 'needs-action')
      throw new Error(`HealthState: status=${pAsUntyped.status} must not carry a remedy`);

    // INV-2
    if (p.components.length !== COMPONENT_ORDER.length)
      throw new Error(`HealthState: expected ${COMPONENT_ORDER.length} components, got ${p.components.length}`);
    for (let i = 0; i < COMPONENT_ORDER.length; i++) {
      if (p.components[i].id !== COMPONENT_ORDER[i])
        throw new Error(`HealthState: components[${i}].id must be "${COMPONENT_ORDER[i]}", got "${p.components[i].id}"`);
    }

    // INV-3
    const expectedPm: PackageManagerId = p.platform === 'windows' ? 'winget' : 'homebrew';
    if (p.packageManager.id !== expectedPm)
      throw new Error(`HealthState: platform=${p.platform} expects packageManager.id="${expectedPm}", got "${p.packageManager.id}"`);

    // Hydrate the frontend-owned description on every gate before assigning.
    // The wire may omit it (current daemon) or ship a stale value — neither
    // should leak to the UI. After this rewrite, every Component on the
    // state has a non-empty, canonical description.
    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = { ...p.packageManager, description: DESCRIPTIONS[p.packageManager.id] };
    this.components     = p.components.map((c) => ({ ...c, description: DESCRIPTIONS[c.id] }));
    this.remedy         = p.remedy;
    this.status         = p.status;

    if (p.status === 'ok') setHealthReady();
    else                   clearHealthCache();
  }

  applyEvent(e: HealthEvent): void {
    switch (e.kind) {
      case 'phase':     this.status = e.phase; return;
      case 'component': this.#patch(e.id, e.patch); return;
      case 'remedy':    this.remedy = e.remedy; return;
      case 'report':    this.apply(e.payload); return;
      default: {
        const _exhaustive: never = e;
        throw new Error(`HealthState: unknown event kind ${JSON.stringify(_exhaustive)}`);
      }
    }
  }

  #patch(id: ComponentId | PackageManagerId, patch: Partial<Component>): void {
    if (id === this.packageManager.id) {
      this.packageManager = { ...this.packageManager, ...patch };
      return;
    }
    const idx = this.components.findIndex((c) => c.id === id);
    if (idx < 0) {
      throw new Error(
        `HealthState: unknown component id "${id}" — not in [${this.packageManager.id}, ${COMPONENT_ORDER.join(', ')}]`
      );
    }
    const next = this.components.slice();
    next[idx] = { ...next[idx], ...patch };
    this.components = next;
  }
}

export const healthState = new HealthState();
