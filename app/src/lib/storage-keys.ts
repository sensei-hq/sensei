/**
 * Centralised localStorage / sessionStorage keys, namespaced by build mode.
 *
 * Dev builds use the `sensei-dev:*` prefix so they don't collide with a
 * production install on the same machine — mirrors how the daemon dirs
 * (~/.sensei-dev vs ~/.sensei) and brew formula labels (sensei-dev vs
 * sensei) are partitioned. Without this, running dev next to prod would
 * have setup-complete, port discovery, app-version, and the health gate
 * stomping each other's state.
 *
 * The namespace is injected at build time by vite.config.ts via the
 * `__SENSEI_NAMESPACE__` define. The fallback is the prod label so a
 * non-vite consumer (e.g. a test runner that hasn't stubbed the define)
 * still gets stable keys.
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
