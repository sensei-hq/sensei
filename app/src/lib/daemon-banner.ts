// What the app-wide strip says while something is wrong with the daemon (pure).
// Rendered by the shared Banner (@sensei/ui) in the root layout: its DB
// connection degraded (it lost the cold-boot race and is self-healing), or the
// settings read failed (appState keeps the last ones and retries). Health is the
// next step — it shows which part is down.
import type { DaemonDbMode } from './health-types.js';

const OPEN_HEALTH = { label: 'Open Health', href: '/health', name: 'health' };

export function daemonBanner(
  mode: DaemonDbMode | undefined,
  configError: string | null,
  down = false,
) {
  if (down) {
    return {
      tone: 'fault' as const,
      title: 'Daemon not answering',
      detail: 'The app shows the last known state. Check that the daemon is running, then open Health.',
      action: OPEN_HEALTH,
    };
  }
  if (mode === 'provisioning') {
    return {
      // A first install still building: a wait, not a fault (and never a refusal).
      tone: 'normal' as const,
      title: 'Setting things up',
      detail: 'The database is still being created — the app fills in as it arrives.',
      action: OPEN_HEALTH,
    };
  }
  if (mode === 'degraded') {
    return {
      // It heals itself: a wait, not a fault (and never a refusal).
      tone: 'normal' as const,
      title: 'Reconnecting to the database',
      detail: 'Some data may be briefly unavailable while the daemon recovers.',
      action: OPEN_HEALTH,
    };
  }
  if (configError) {
    return {
      tone: 'fault' as const,
      title: 'Could not read your settings from the daemon',
      detail: `${configError} — showing the last ones; trying again.`,
      action: OPEN_HEALTH,
    };
  }
  return null;
}
