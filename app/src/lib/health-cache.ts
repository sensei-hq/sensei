/**
 * Sole owner of the `sensei:health` sessionStorage cache.
 *
 * Every read and write of this key in the app goes through this module.
 * Two callers:
 *   - `HealthState.apply()` writes 'ready' on ok, clears otherwise.
 *   - `hooks.client.ts` calls `initHealthCache()` once at module init (cold
 *     start, before any HealthState code can run) and `isHealthReady()` in
 *     reroute. It cannot import HealthState directly because that module
 *     uses `$state` runes, which crash the Svelte runtime if loaded before
 *     the SvelteKit hook entry point.
 *
 * This file deliberately has no Svelte-rune dependencies so hooks.client.ts
 * can use it safely.
 */

const KEY = 'sensei:health';
const VALUE_READY = 'ready';

/** True when `VITE_BYPASS_HEALTH=true` is in effect for this build. */
export function isHealthBypass(): boolean {
  return import.meta.env.VITE_BYPASS_HEALTH === 'true';
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
 * - Bypass mode: pre-populate 'ready' so reroute passes without a /health hop.
 * - Normal mode: clear any stale 'ready' that survived hot-reload in tauri:dev.
 *   (WKWebView clears sessionStorage on app exit but not across hot reloads.)
 */
export function initHealthCache(): void {
  if (isHealthBypass()) setHealthReady();
  else clearHealthCache();
}
