/**
 * Bootstrap client — talks to Tauri bootstrap commands or daemon health API.
 *
 * Two modes:
 * - Tauri mode: invoke('run_bootstrap') → full prereq check via bootstrap crate
 * - HTTP mode: fetch('/api/health/components') → daemon reports component status
 */

import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

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

async function hasTauri(): Promise<boolean> {
  try {
    const mod = await import('@tauri-apps/api/core');
    return !!mod.invoke;
  } catch {
    return false;
  }
}

// ── Bootstrap API ────────────────────────────────────────────────────────────

/**
 * Run the full bootstrap check. Tries daemon API first (fast path),
 * falls back to Tauri commands if daemon is unreachable.
 */
export async function runBootstrap(): Promise<BootstrapResult> {
  // Fast path: daemon is running, ask it for component status
  try {
    const api = senseiApi(appState.port);
    const resp = await api.getComponents();
    if (resp && 'data' in resp) {
      return resp as unknown as BootstrapResult;
    }
  } catch {
    // Daemon unreachable — fall through to Tauri
  }

  // Slow path: Tauri bootstrap commands
  if (await hasTauri()) {
    return tauriInvoke<BootstrapResult>('run_bootstrap');
  }

  // No Tauri, no daemon — return all-failed
  return {
    components: [
      { name: 'daemon', state: { state: 'failed', error: 'Not reachable' }, version: null, detail: null },
    ],
    hardware: { ram_gb: 0, cpu_cores: 0, gpu: null, metal_support: false, recommended_tier: 'minimum' },
    ready: false,
  };
}

/** Install a component by name. Requires Tauri. */
export async function installComponent(name: string): Promise<ComponentStatus> {
  return tauriInvoke<ComponentStatus>('install_component', { name });
}

/** Start a service by name. Requires Tauri. */
export async function startComponent(name: string): Promise<ComponentStatus> {
  return tauriInvoke<ComponentStatus>('start_component', { name });
}

/** Create the sensei database. Requires Tauri. */
export async function createDatabase(): Promise<ComponentStatus> {
  return tauriInvoke<ComponentStatus>('create_database');
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
