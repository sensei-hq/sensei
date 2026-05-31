/**
 * Centralised localStorage / sessionStorage keys, namespaced by app id.
 *
 * The `sensei:` prefix exists so sensei's keys don't collide with other apps
 * sharing a WebView storage origin. The previous dev/prod split (which used
 * `sensei-dev:` for dev builds) is gone — there is one namespace now.
 *
 * The namespace is injected at build time by vite.config.ts via the
 * `__SENSEI_NAMESPACE__` define. The fallback exists so a non-vite consumer
 * (e.g. a test runner that hasn't stubbed the define) still gets stable keys.
 */

declare const __SENSEI_NAMESPACE__: string;
const NS: string = typeof __SENSEI_NAMESPACE__ !== 'undefined' ? __SENSEI_NAMESPACE__ : 'sensei';

export const STORAGE_KEYS = {
  port:           `${NS}:port`,
  setupComplete:  `${NS}:setup-complete`,
  userName:       `${NS}:userName`,
  appVersion:     `${NS}:app-version`,
  health:         `${NS}:health`,
  testMode:       `${NS}:test-mode`,
} as const;

/** Re-exported namespace for any consumer that needs to build a key
 *  outside the canonical set above. */
export const STORAGE_NAMESPACE = NS;
