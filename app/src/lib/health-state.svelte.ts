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

function emptyComponent(id: ComponentId): Component {
  const d = COMPONENT_DEFAULTS[id];
  return { id, label: d.label, note: d.note, status: 'pending', version: null, detail: null, installingVerb: d.installingVerb };
}

/** Deterministic default — status='checking' so the UI never flashes 'ok' pre-apply. */
export const emptyPayload: HealthPayload = {
  version: '',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: 'which brew', status: 'pending', version: null, detail: null, installingVerb: 'installing' },
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
    const payload = await this.#transport.check();
    this.apply(payload);
    if (payload.status === 'ok') return;
    await this.#transport.resolve(payload, (ev) => this.applyEvent(ev));
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

    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = p.packageManager;
    this.components     = p.components;
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
