/**
 * Sole owner of the `sensei:health` sessionStorage cache.
 *
 * Every read and write of this key in the app goes through this module.
 * Two callers:
 *   - `HealthState.apply()` writes 'ready' on ok, clears otherwise.
 *   - `hooks.client.ts` calls `initHealthCache()` once at module init
 *     (cold start, before any HealthState code can run) and
 *     `isHealthReady()` in reroute. It cannot import HealthState directly
 *     because that module uses `$state` runes, which crash the Svelte
 *     runtime if loaded before the SvelteKit hook entry point.
 *
 * This file deliberately has no Svelte-rune dependencies so hooks.client.ts
 * can use it safely.
 *
 * Bypass semantics:
 *   The health gate is bypassed when the app is NOT running inside Tauri
 *   (= no `window.__TAURI__`). Tauri injects its global before any user
 *   script runs (`withGlobalTauri: true` in tauri.conf.json), so by the
 *   time `initHealthCache()` is called the global is reliably present in
 *   Tauri builds and reliably absent in plain `vite dev`. No env vars,
 *   no build-time constants, no leak surface.
 */

const KEY = 'sensei:health';
const VALUE_READY = 'ready';

/** True when the app is running inside a Tauri webview.
 *  Tauri sets `window.__TAURI__` via a script that runs before any
 *  bundle code. In `vite dev` / `vite preview` it is absent. */
function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined'
    && !!(window as { __TAURI__?: unknown }).__TAURI__;
}

/** True when the health check should be bypassed (no Tauri sidecar to
 *  answer it, so there is nothing to check). */
export function isHealthBypass(): boolean {
  return !hasTauriRuntime();
}

/** Reroute consults this to decide whether to redirect to /health. */
export function isHealthReady(): boolean {
  if (typeof sessionStorage === 'undefined') return false;
  return sessionStorage.getItem(KEY) === VALUE_READY;
}

/** HealthState.apply() calls this when status === 'ok'. */
export function setHealthReady(): void {
  if (typeof sessionStorage === 'undefined') return;
  sessionStorage.setItem(KEY, VALUE_READY);
}

/** HealthState.apply() / verify() call this when status is not 'ok'. */
export function clearHealthCache(): void {
  if (typeof sessionStorage === 'undefined') return;
  sessionStorage.removeItem(KEY);
}

/**
 * Cold-start initializer. Runs once from hooks.client.ts at module init,
 * before any reroute or HealthState code.
 *
 * - No Tauri (browser dev / static preview): pre-populate 'ready' so the
 *   reroute hook passes without a /health hop.
 * - Tauri: clear any stale 'ready' that survived a hot reload in
 *   tauri:dev. (WKWebView clears sessionStorage on app exit but not
 *   across hot reloads.)
 */
export function initHealthCache(): void {
  if (isHealthBypass()) setHealthReady();
  else clearHealthCache();
}
