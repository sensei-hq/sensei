import { healthState } from "$lib/health-state.svelte.js";
import { wizardState } from "$lib/wizard-state.svelte.js";

const ALWAYS_REACHABLE = new Set(["/health", "/logs", "/upgrade"]);

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;
  if (!ALWAYS_REACHABLE.has(path) && !healthState.isOk) return "/health";
  // TEMP: setup gate disabled while debugging Phase 0 in the live app (2026-05-27).
  // Restore once setup wizard regressions tracked in docs/backlog.md are resolved.
  // if (!ALWAYS_REACHABLE.has(path) && !path.startsWith("/setup") && !wizardState.isOk) return "/setup/welcome";
  void wizardState;
  return undefined;
}
