import { isSettingsPath } from './settings-path.js';

/**
 * The boot gate: where a navigation must go instead, given the app's state.
 *
 * Pure — the state is passed in — so the root layout's load can ask it on every
 * navigation. It used to be the `reroute` hook, which SvelteKit caches per URL
 * for the whole session on the promise that reroute is pure. Ours depends on
 * health and setup, so once `/` resolved to the setup screen it stayed there
 * after setup completed, until the app restarted.
 */

const ALWAYS_REACHABLE = new Set(['/health', '/logs', '/upgrade']);

// A workspace window loads `/workspace/{id}/...` in its own fresh webview heap,
// where healthState starts 'checking' (isOk=false) and setup state is
// unhydrated. It is only ever opened from an already-healthy main window, so it
// must bypass the boot gates — otherwise the gate bounces the fresh window to
// /health and the workspace never renders. The settings window is the same
// kind of window (user, 2026-10-27). This matches the singular `/workspace/`
// window routes, not the observatory's plural `/workspaces` list (still gated).
function isReachable(path: string): boolean {
  return ALWAYS_REACHABLE.has(path) || path.startsWith('/workspace/') || isSettingsPath(path);
}

export function gateTarget(
  path: string,
  state: { healthOk: boolean; setupOk: boolean },
): string | undefined {
  if (isReachable(path)) return undefined;
  if (!state.healthOk) return '/health';
  if (!path.startsWith('/setup') && !state.setupOk) return '/setup/welcome';
  return undefined;
}
