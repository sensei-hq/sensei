/**
 * Daemon lifecycle management for the setup wizard.
 *
 * Handles: detect → start → verify for senseid, sensei-cli, sensei-mcp.
 * Uses Tauri shell commands when available, falls back to daemon API.
 */

import { senseiApi } from '$lib/api.js';
import { getPort } from '$lib/appstate.svelte.js';
import type { ComponentStatus } from './types.js';

export type ComponentState = 'checking' | 'missing' | 'installing' | 'stopped' | 'starting' | 'ready' | 'error';

export interface ComponentInfo {
  id: string;
  name: string;
  icon: string;
  state: ComponentState;
  version: string | null;
  error: string | null;
}

const INITIAL_COMPONENTS: ComponentInfo[] = [
  { id: 'cli',    name: 'sensei-cli',    icon: '令', state: 'checking', version: null, error: null },
  { id: 'mcp',    name: 'MCP bridge',    icon: '橋', state: 'checking', version: null, error: null },
  { id: 'daemon', name: 'sensei-daemon', icon: '守', state: 'checking', version: null, error: null },
];

/** Check if Tauri shell API is available (running in desktop, not browser). */
async function hasTauri(): Promise<boolean> {
  try {
    const mod = await import('@tauri-apps/api/core');
    return !!mod.invoke;
  } catch {
    return false;
  }
}

/** Run a shell command via Tauri and return stdout. */
async function tauriExec(program: string, args: string[]): Promise<{ stdout: string; code: number }> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    // Use Tauri's shell execute (requires shell plugin or custom command)
    // Fallback: use invoke with a custom Rust command
    const result = await invoke<{ stdout: string; code: number }>('run_command', { program, args });
    return result;
  } catch {
    return { stdout: '', code: -1 };
  }
}

/**
 * Check and start all components. Returns a reactive callback that emits updates.
 *
 * Usage:
 *   let components = $state(getInitialComponents());
 *   onMount(() => checkComponents((updated) => { components = updated; }));
 */
export function getInitialComponents(): ComponentInfo[] {
  return INITIAL_COMPONENTS.map(c => ({ ...c }));
}

export async function checkComponents(
  onUpdate: (components: ComponentInfo[]) => void
): Promise<void> {
  const components = getInitialComponents();
  const emit = () => onUpdate([...components]);
  emit();

  const port = getPort();
  const api = senseiApi(port);

  // ── Step 1: Check if daemon is running ────────────────────
  let daemonRunning = false;
  try {
    const health = await api.getHealth();
    if (health?.ok) {
      daemonRunning = true;
      const daemonComp = components.find(c => c.id === 'daemon')!;
      daemonComp.state = 'ready';
      daemonComp.version = String(health.version ?? '');
      emit();
    }
  } catch {
    // Daemon not responding
  }

  // ── Step 2: If daemon is running, ask it about CLI + MCP ──
  if (daemonRunning) {
    try {
      const data = await api.getComponents();
      for (const comp of data.components ?? []) {
        const local = components.find(c => c.id === comp.id);
        if (local) {
          local.state = comp.status === 'ready' ? 'ready' : comp.status === 'missing' ? 'missing' : 'checking';
          local.version = comp.version ?? null;
        }
      }
      emit();
      return; // All done — daemon told us everything
    } catch {
      // Failed to get component details — check manually
    }
  }

  // ── Step 3: Daemon not running — try to detect and start ──
  const isTauri = await hasTauri();

  // Check CLI
  const cliComp = components.find(c => c.id === 'cli')!;
  if (isTauri) {
    const result = await tauriExec('sensei', ['--version']);
    if (result.code === 0) {
      cliComp.state = 'ready';
      cliComp.version = result.stdout.trim();
    } else {
      cliComp.state = 'missing';
    }
  } else {
    // In browser dev mode, we can't check PATH. Mark as unknown.
    cliComp.state = daemonRunning ? 'ready' : 'missing';
  }
  emit();

  // Check MCP bridge
  const mcpComp = components.find(c => c.id === 'mcp')!;
  if (isTauri) {
    const result = await tauriExec('sensei-mcp', ['--version']);
    if (result.code === 0) {
      mcpComp.state = 'ready';
      mcpComp.version = result.stdout.trim();
    } else {
      mcpComp.state = 'missing';
    }
  } else {
    mcpComp.state = daemonRunning ? 'ready' : 'missing';
  }
  emit();

  // ── Step 4: Try to start daemon if found but not running ──
  const daemonComp = components.find(c => c.id === 'daemon')!;
  if (!daemonRunning) {
    if (isTauri) {
      // Try to find and start senseid
      const check = await tauriExec('which', ['senseid']);
      if (check.code === 0) {
        daemonComp.state = 'starting';
        emit();

        // Start daemon
        await tauriExec('senseid', ['start', '--port', String(port)]);

        // Wait for it to be ready
        for (let i = 0; i < 20; i++) {
          await new Promise(r => setTimeout(r, 500));
          try {
            const health = await api.getHealth();
            if (health?.ok) {
              daemonComp.state = 'ready';
              daemonComp.version = String(health.version ?? '');
              emit();

              // Now that daemon is running, re-check CLI + MCP via API
              try {
                const data = await api.getComponents();
                for (const comp of data.components ?? []) {
                  const local = components.find(c => c.id === comp.id);
                  if (local && local.state !== 'ready') {
                    local.state = comp.status === 'ready' ? 'ready' : 'missing';
                    local.version = comp.version ?? null;
                  }
                }
                emit();
              } catch { /* non-fatal */ }
              return;
            }
          } catch { /* not ready yet */ }
        }
        // Failed to start
        daemonComp.state = 'error';
        daemonComp.error = 'Daemon did not start within 10s';
        emit();
      } else {
        daemonComp.state = 'missing';
        emit();
      }
    } else {
      // Browser dev mode — can't start daemon, show instructions
      daemonComp.state = 'stopped';
      daemonComp.error = `Start with: senseid start --port ${port}`;
      emit();
    }
  }
}

/** Install sensei via homebrew. Only works in Tauri. */
export async function installViaBrew(
  onUpdate: (status: string) => void
): Promise<boolean> {
  const isTauri = await hasTauri();
  if (!isTauri) {
    onUpdate('Install manually: brew install mizukisu/tap/sensei');
    return false;
  }

  onUpdate('Installing via homebrew...');
  const result = await tauriExec('brew', ['install', 'mizukisu/tap/sensei']);
  if (result.code === 0) {
    onUpdate('Installed successfully');
    return true;
  } else {
    onUpdate(`Install failed: ${result.stdout}`);
    return false;
  }
}
