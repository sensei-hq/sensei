// app/src/lib/health-types.ts

// ── Closed enums ─────────────────────────────────────────────────────────

export type Platform        = 'macos' | 'linux' | 'windows';
export type HealthStatus    = 'checking' | 'resolving' | 'ok' | 'needs-action';
export type ComponentStatus =
  | 'pending' | 'checking' | 'installing' | 'ready' | 'failed';

export type ComponentId      = 'postgres' | 'ollama' | 'sensei' | 'database' | 'daemon';
export type PackageManagerId = 'homebrew' | 'winget';

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
