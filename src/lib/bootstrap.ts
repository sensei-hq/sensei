/**
 * Bootstrap client — talks to Tauri bootstrap commands or daemon health API.
 *
 * Two modes:
 * - Tauri mode: invoke('run_bootstrap') → full prereq check via bootstrap crate
 * - HTTP mode: fetch('/api/health/components') → daemon reports component status
 */

// senseiApi is used by post-bootstrap screens (wizard, observatory) — not here.
// Bootstrap always uses the Tauri sidecar directly.

// ── Types (match bootstrap crate types) ──────────────────────────────────────

export interface ComponentStatus {
  name: string;
  state: ComponentState;
  version: string | null;
  detail: string | null;
}

export type ComponentState =
  | { state: 'detecting' }
  | { state: 'installing' }
  | { state: 'starting' }
  | { state: 'upgrading' }
  | { state: 'pulling'; progress_pct: number; size_mb: number }
  | { state: 'ready' }
  | { state: 'failed'; error: string }
  | { state: 'skipped' };

export interface HardwareInfo {
  ram_gb: number;
  cpu_cores: number;
  gpu: string | null;
  metal_support: boolean;
  recommended_tier: 'minimum' | 'recommended' | 'full';
}

export interface BootstrapResult {
  components: ComponentStatus[];
  hardware: HardwareInfo;
  ready: boolean;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

export function stateLabel(s: ComponentState): string {
  return s.state;
}

export function isReady(s: ComponentState): boolean {
  return s.state === 'ready';
}

export function isFailed(s: ComponentState): boolean {
  return s.state === 'failed';
}

export function errorMessage(s: ComponentState): string | null {
  return s.state === 'failed' ? s.error : null;
}

// ── Tauri detection ──────────────────────────────────────────────────────────

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** True when running inside Tauri app, false in browser. */
export function hasTauri(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI__;
}

// ── Bootstrap API ────────────────────────────────────────────────────────────

/**
 * Run the full bootstrap check via Tauri sidecar.
 *
 * Always uses the sidecar (not the daemon API) because during bootstrap
 * the daemon may not be running yet. The daemon fast-path is used by
 * post-bootstrap screens that know the daemon is up.
 */
export async function runBootstrap(): Promise<BootstrapResult> {
  // Browser (no Tauri) → mock data for development/testing
  if (!hasTauri()) {
    const { mockBootstrapPartial } = await import('./mock-data.js');
    return mockBootstrapPartial;
  }

  return tauriInvoke<BootstrapResult>('run_bootstrap');
}

/** Get hardware info. Requires Tauri. */
export async function detectHardware(): Promise<HardwareInfo> {
  return tauriInvoke<HardwareInfo>('detect_hardware');
}

/** List installed Ollama models. Requires Tauri. */
export async function listModels(): Promise<string[]> {
  return tauriInvoke<string[]>('list_models');
}

/** Check which models are missing. Requires Tauri. */
export async function missingModels(): Promise<string[]> {
  return tauriInvoke<string[]>('missing_models');
}

/** Install prerequisites via platform provider. Requires Tauri. */
export async function installPrerequisites(): Promise<void> {
  return tauriInvoke<void>('install_prerequisites');
}

/** Start services sequentially. Requires Tauri. */
export async function startServices(): Promise<void> {
  return tauriInvoke<void>('start_services');
}

/** Run database setup pipeline. Requires Tauri. */
export async function setupDatabase(): Promise<void> {
  return tauriInvoke<void>('setup_database');
}

/** Get platform info from the backend. Requires Tauri. */
export async function getPlatform(): Promise<any> {
  return tauriInvoke<any>('get_platform');
}

/**
 * Listen for bootstrap events from the Tauri backend.
 * Dispatches to the provided handler (which should be bs.handleEvent).
 * Returns an unlisten function.
 */
export async function listenBootstrapEvents(
  handler: (event: { action: 'update' | 'set'; entity: 'gate' | 'phase'; id: string; data: Record<string, unknown> }) => void,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<any>('bootstrap', (event) => {
    handler(event.payload);
  });
  return unlisten;
}

