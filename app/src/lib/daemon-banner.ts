// What the app-wide strip says while something is wrong with the daemon (pure).
// Rendered by the shared Banner (@sensei/ui) in the root layout: its DB
// connection degraded (it lost the cold-boot race and is self-healing), or the
// settings read failed (appState keeps the last ones and retries). Health is the
// next step — it shows which part is down.
import type { DaemonDbMode } from './health-types.js';

const OPEN_HEALTH = { label: 'Open Health', href: '/health', name: 'health' };

export function daemonBanner(mode: DaemonDbMode | undefined, configError: string | null) {
  if (mode === 'degraded') {
    return {
      tone: 'refusal' as const,
      title: 'Reconnecting to the database',
      detail: 'Some data may be briefly unavailable while the daemon recovers.',
      action: OPEN_HEALTH,
    };
  }
  if (configError) {
    return {
      tone: 'refusal' as const,
      title: 'Could not read your settings from the daemon',
      detail: `${configError} — showing the last ones; trying again.`,
      action: OPEN_HEALTH,
    };
  }
  return null;
}
