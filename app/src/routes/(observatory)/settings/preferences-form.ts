import type { PreferencesData } from '$lib/setup/contracts.js';

/** Defaults kept in one place so a missing / corrupt config value renders
 *  the same form the wizard would. */
export const DEFAULT_PREFERENCES: PreferencesData = {
  displayName: '',
  contributeLearnings: true,
  reviewBeforeShare: true,
  shareSchedule: 'weekly-saturday',
  downloadCollective: 'weekly',
  correctionAggressiveness: 'balanced',
  digestCadence: 'daily',
  nudgeOnRegression: true,
  anonymizedTelemetry: false,
  showWelcome: true,
};

/** Pure: parse the daemon's config map into a preferences form. The wizard
 *  writes prefs as a JSON blob under `setup.preferences` PLUS a plain
 *  `user_name` for the greeting. Corrupt JSON falls back to the defaults
 *  (rather than throwing) so the settings page always renders. */
export function toPreferencesForm(config: Record<string, string>): PreferencesData {
  const raw = config['setup.preferences'];
  const parsed = raw ? safeJsonParse<Partial<PreferencesData>>(raw) : undefined;
  const merged: PreferencesData = { ...DEFAULT_PREFERENCES, ...(parsed ?? {}) };
  // `user_name` is authoritative for displayName when both exist — the
  // wizard writes it as the ground truth for downstream greeters.
  if (config['user_name']) merged.displayName = config['user_name'];
  return merged;
}

/** Pure: serialize a preferences form back into the config-key/value shape
 *  `setConfig` expects. Mirrors the wizard's `preferences` commit handler
 *  so both call sites write the same keys — the greeting still reads from
 *  `user_name`, and the wizard still parses `setup.preferences`. */
export function fromPreferencesForm(form: PreferencesData): Record<string, string> {
  return {
    'setup.preferences': JSON.stringify(form),
    'user_name': form.displayName,
  };
}

function safeJsonParse<T>(raw: string): T | undefined {
  try { return JSON.parse(raw) as T; } catch { return undefined; }
}
