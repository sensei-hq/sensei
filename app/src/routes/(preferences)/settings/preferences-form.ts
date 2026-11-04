import type { PreferencesData } from '$lib/setup/contracts.js';
import { GLYPHS_KEY, glyphMode } from '$lib/glyphs';
import { DEFAULT_PREFERENCES } from '$lib/setup/preferences.js';

/** Re-exported: the defaults live in `$lib/setup/preferences`, shared with the wizard. */
export { DEFAULT_PREFERENCES };

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
  // Its own key (#133), so the rails read it without parsing this blob.
  merged.glyphs = glyphMode(config);
  return merged;
}

/** The stored preferences as the settings screen may edit them (D83 batch A,
 *  audit row 73): the form, or — when the stored JSON is corrupt — a failure with
 *  its reason, so the screen shows a fault and blocks saving instead of showing
 *  the defaults a save would write over the real values. */
export type PreferencesRead =
  | { ok: true; form: PreferencesData }
  | { ok: false; reason: string };

/** Pending (D83 batch A phase B): today's behaviour reads corrupt JSON as the defaults. */
export function readPreferencesForm(config: Record<string, string>): PreferencesRead {
  return { ok: true, form: toPreferencesForm(config) };
}

/** Pure: serialize a preferences form back into the config-key/value shape
 *  `appState.setConfigs` expects. Mirrors the wizard's `preferences` commit handler
 *  so both call sites write the same keys — the greeting still reads from
 *  `user_name`, and the wizard still parses `setup.preferences`. */
export function fromPreferencesForm(form: PreferencesData): Record<string, string> {
  return {
    'setup.preferences': JSON.stringify(form),
    'user_name': form.displayName,
    [GLYPHS_KEY]: form.glyphs,
  };
}

function safeJsonParse<T>(raw: string): T | undefined {
  try { return JSON.parse(raw) as T; } catch { return undefined; }
}
