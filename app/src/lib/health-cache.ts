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
 *   The health gate is bypassed when the bundle is NOT built inside a
 *   Tauri context — i.e. `bun run dev` or a static preview, where no Tauri
 *   sidecar exists to answer the health check. The `__SENSEI_HAS_TAURI__`
 *   constant is injected by vite.config.ts from the TAURI_PLATFORM /
 *   TAURI_ENV_PLATFORM / TAURI_ENV_DEBUG env vars that Tauri itself sets
 *   on `tauri dev` / `tauri build`. No custom env var is involved, so
 *   nothing can leak.
 */

declare const __SENSEI_HAS_TAURI__: boolean;

const KEY = 'sensei:health';
const VALUE_READY = 'ready';

/** True when the bundle was built inside a Tauri context (= a Tauri
 *  sidecar will be present at runtime to answer the health check).
 *
 *  vite.config.ts always substitutes `__SENSEI_HAS_TAURI__` to a literal
 *  `true`/`false` at compile time, so in any real build this returns a
 *  definite answer. The `typeof ... === 'undefined'` branch only fires in
 *  vitest where vite's `define` doesn't apply — we default to `true`
 *  there (= no bypass) so tests exercise the production code path by
 *  default. Bypass-specific tests opt in by stubbing the global. */
function hasTauriBuild(): boolean {
  return typeof __SENSEI_HAS_TAURI__ === 'undefined' || __SENSEI_HAS_TAURI__;
}

/** True when the health check should be bypassed (browser-only mode). */
export function isHealthBypass(): boolean {
  return !hasTauriBuild();
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
 * - Bypass mode (browser-only build): pre-populate 'ready' so reroute
 *   passes without a /health hop.
 * - Tauri build: clear any stale 'ready' that survived hot-reload in
 *   tauri:dev. (WKWebView clears sessionStorage on app exit but not
 *   across hot reloads.)
 */
export function initHealthCache(): void {
  if (isHealthBypass()) setHealthReady();
  else clearHealthCache();
}
