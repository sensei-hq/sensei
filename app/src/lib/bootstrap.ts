/**
 * Bootstrap client — talks to Tauri bootstrap commands.
 *
 * The single `checkAndFixBootstrap()` call replaces the old three-phase API
 * (runBootstrap / installPrerequisites / startServices / setupDatabase).
 * Progress arrives via `listenBootstrapEvents()`; the final `BootstrapReport`
 * is emitted as a "bootstrap-report" Tauri event when the engine finishes.
 */

// ── Types (match bootstrap crate types) ──────────────────────────────────────

export interface HardwareInfo {
  ram_gb: number;
  cpu_cores: number;
  gpu: string | null;
  metal_support: boolean;
  recommended_tier: 'minimum' | 'recommended' | 'full';
}

export type GateStatus =
  | { status: 'Checking' }
  | { status: 'Installing' }
  | { status: 'Starting' }
  | { status: 'Ready'; version: string | null; detail: string | null }
  | { status: 'Failed'; error: string };

export interface GateReport {
  id: string;
  status: GateStatus;
  fix_attempted: boolean;
  fix_detail: string | null;
}

export interface HumanAction {
  component_id: string;
  title: string;
  command: string;
  url: string | null;
}

export interface BootstrapReport {
  gates: GateReport[];
  all_ok: boolean;
  blocked_on: HumanAction | null;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/** True when running inside Tauri app, false in browser. */
export function hasTauri(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI__;
}

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// ── Bootstrap API ────────────────────────────────────────────────────────────

/**
 * Run the full check-and-fix bootstrap pipeline.
 *
 * Checks all prerequisites, then fixes what's broken — all handled by the
 * BootstrapEngine. Returns immediately; progress arrives via listenBootstrapEvents().
 * The final BootstrapReport is emitted as a "bootstrap-report" Tauri event.
 *
 * Falls back to a no-op in browser mode (mock data is applied separately).
 */
export async function checkAndFixBootstrap(): Promise<void> {
  if (!hasTauri()) return;
  return tauriInvoke<void>('check_and_fix_bootstrap');
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

/**
 * Return the daemon port for the current runtime mode.
 *
 * Delegates to SenseiConfig::from_env() in the sidecar — the single source
 * of truth. Returns 7744 (prod) or 7745 (dev). Call once at app startup to
 * initialise appState.port before any daemon API calls.
 *
 * Falls back to 7744 if running outside Tauri (browser dev mode).
 */
export async function getDaemonPort(): Promise<number> {
  if (!hasTauri()) return 7744;
  return tauriInvoke<number>('get_daemon_port');
}

/** Get platform info from the backend. Requires Tauri. */
export async function getPlatform(): Promise<any> {
  return tauriInvoke<any>('get_platform');
}

/**
 * Listen for bootstrap gate/phase events from the Tauri backend.
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

/**
 * Listen for the final BootstrapReport emitted when the engine finishes.
 * Returns an unlisten function.
 */
export async function listenBootstrapReport(
  handler: (report: BootstrapReport) => void,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<BootstrapReport>('bootstrap-report', (event) => {
    handler(event.payload);
  });
  return unlisten;
}
