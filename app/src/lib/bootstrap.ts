/**
 * Tauri detection helper.
 *
 * What was here was an orphaned client surface from an earlier bootstrap
 * design (checkAndFixBootstrap + listenBootstrapEvents + listenBootstrapReport
 * + detectHardware/listModels/missingModels/getDaemonPort/getPlatform). None
 * had production callers; `check_and_fix_bootstrap` no longer existed on
 * the Rust side and would have thrown "command not found" at runtime if
 * invoked. Deleted in the 2026-05-15 audit cleanup.
 */

/** True when running inside the Tauri app shell, false in browser dev mode. */
export function hasTauri(): boolean {
  return typeof window !== 'undefined' && !!(window as { __TAURI__?: unknown }).__TAURI__;
}
