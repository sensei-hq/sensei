/**
 * Sole owner of the `sensei:health` sessionStorage cache.
 *
 * Pure module — no `$state` runes, no Svelte runtime dependencies — so
 * `hooks.ts`/`hooks.client.ts` can safely import it at module-init time
 * (importing a rune module from hooks causes a blank screen).
 *
 * Lifecycle:
 *   - `initHealthCache()` runs once at hook module-init: clears the key
 *     so a stale 'ready' from a previous session (or surviving a tauri
 *     hot reload) cannot bypass the gate on a fresh launch.
 *   - `HealthState.apply()` writes 'ready' via `setHealthReady()` when
 *     status becomes ok; clears via `clearHealthCache()` otherwise.
 *   - Reroute consults `isHealthReady()` to decide whether the gate is
 *     open. Anything not on the exempt list and not ready goes to
 *     `/health`.
 */

import { STORAGE_KEYS } from './storage-keys.js';
const KEY = STORAGE_KEYS.health;
const VALUE_READY = 'ready';

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

/** Cold-start initializer. Always clears the key — fresh launches must
 *  re-prove health, and a stale 'ready' that survived a Tauri hot
 *  reload must not bypass the new check. */
export function initHealthCache(): void {
  clearHealthCache();
}
