// app/src/lib/health-types.ts

// ── Closed enums ─────────────────────────────────────────────────────────

export type Platform        = 'macos' | 'linux' | 'windows';
export type HealthStatus    = 'checking' | 'resolving' | 'ok' | 'needs-action';
export type ComponentStatus =
  | 'pending' | 'checking' | 'installing' | 'ready' | 'failed';

export type ComponentId      = 'postgres' | 'ollama' | 'sensei' | 'database' | 'daemon';
export type PackageManagerId = 'homebrew' | 'winget';

/** The daemon's own DB-connection mode, reported by the daemon that served the
 *  payload. Distinct from the Postgres component status: 'degraded' means the
 *  daemon has no working pool yet (e.g. it lost the cold-boot race) and is
 *  self-healing, even if Postgres itself probes 'ready'. Absent when the payload
 *  wasn't produced by a daemon. */
export type DaemonDbMode = 'full' | 'degraded';

export const COMPONENT_ORDER: readonly ComponentId[] =
  ['postgres', 'ollama', 'sensei', 'database', 'daemon'] as const;

// ── Component (both ledger rows and package manager use this shape) ──────

export interface Component {
  id: ComponentId | PackageManagerId;
  label: string;
  note: string | null;
  status: ComponentStatus;
  version: string | null;
  detail: string | null;
  /** Verb shown when status is 'installing'. Source of truth is the Rust
   *  `DependencySpec::installing_verb` — carried on the wire so this file
   *  doesn't duplicate the mapping. */
  installingVerb: string;
  /** Poetic one-liner per gate, shown beneath the label in the ledger.
   *  Frontend-only: hydrated by HealthState.apply()/#patch() from the
   *  DESCRIPTIONS map. The wire never has to carry it; if it does, the
   *  state still overwrites with the canonical line so copy stays in
   *  one place. */
  description: string;
}

// ── Remedy — opaque strings produced by the Rust crate ───────────────────

export interface Remedy {
  message: string;
  script: string;
  url: string | null;
}

// ── Discriminated union: status === 'needs-action' ⇔ remedy !== null ─────

type PayloadBase = {
  version: string;
  uptimeSeconds: number;
  platform: Platform;
  packageManager: Component;
  components: Component[];
  /** Present only on daemon-served payloads; omitted by the bootstrap sidecar. */
  daemonDbMode?: DaemonDbMode;
};

export type HealthPayload =
  | (PayloadBase & { status: Exclude<HealthStatus, 'needs-action'>; remedy: null })
  | (PayloadBase & { status: 'needs-action'; remedy: Remedy });

// ── Streaming events ─────────────────────────────────────────────────────

export type HealthEvent =
  | { kind: 'phase';     phase: Extract<HealthStatus, 'checking' | 'resolving'> }
  | { kind: 'component'; id: ComponentId | PackageManagerId; patch: Partial<Component> }
  | { kind: 'remedy';    remedy: Remedy }
  | { kind: 'report';    payload: HealthPayload };
