/**
 * Centralised localStorage keys.
 *
 * Keep this list short and deliberate. Every entry here is mutable user
 * state that needs to survive a session. Build-time constants (port,
 * version) do NOT belong here — they should be baked in at compile time.
 *
 * Previously this module also held `port`, `health`, and `testMode`:
 *   - `port` was a runtime override for the daemon port. A stale value
 *     from a previous session silently routed every fetch to the wrong
 *     port. Removed — port is build-time.
 *   - `health` was a sessionStorage cache for the boot gate. Nothing
 *     read it; HealthState is the authoritative source. Removed.
 *   - `testMode` belonged to a dead mock-data module. Removed.
 */

declare const __SENSEI_NAMESPACE__: string;
const NS = __SENSEI_NAMESPACE__;

export const STORAGE_KEYS = {
  setupComplete:  `${NS}:setup-complete`,
  userName:       `${NS}:userName`,
  /** Set by the upgrade flow before relaunch; cleared on the post-upgrade
   *  page once the new sidecar is running. The value is the *staged*
   *  version — used to detect "the binary I'm running now is older than
   *  the one we just installed" and trigger the upgrade screen. */
  appVersion:     `${NS}:app-version`,
  /** Observatory · Workspaces grid/list view preference ("grid" | "list").
   *  Defaults to "grid"; survives reloads so a return visit lands in the
   *  same layout. No settings-row endpoint exists for this today. */
  workspacesView:   `${NS}:workspaces:view`,
  /** Workspace · Diagrams: whether "What it's for" is open ("open" | "closed"). */
  diagramsAbout:    `${NS}:diagrams:about`,
  /** Workspace · Diagrams: whether the workspace stats band is shown ("open" | "closed"). */
  diagramsStats:    `${NS}:diagrams:stats`,
  /** Workspace · Diagrams: whether the controls band is shown ("open" | "closed"). */
  diagramsControls: `${NS}:diagrams:controls`,
  /** Workspace · Diagrams: whether the side panel is shown ("open" | "closed"). */
  diagramsSide:     `${NS}:diagrams:side`,
} as const;

/** The active Storage, or null when there isn't a usable one. Feature-detects
 *  the methods rather than just `typeof localStorage` so a build-time / SSR
 *  context — or a test runner whose experimental `localStorage` global is a
 *  non-functional stub — resolves cleanly instead of throwing. */
export function browserStorage(): Storage | null {
  const ls = (globalThis as { localStorage?: Storage }).localStorage;
  return ls && typeof ls.getItem === 'function' && typeof ls.setItem === 'function' ? ls : null;
}

/** Re-exported namespace for any consumer that needs to build a key
 *  outside the canonical set above. */
export const STORAGE_NAMESPACE = NS;
