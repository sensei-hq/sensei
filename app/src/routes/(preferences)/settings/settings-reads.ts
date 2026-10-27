// The Settings pages' reads, each the daemon's answer or why it failed (#288).
// A failed read is shown as one — never an empty list ("none detected", "none
// installed") and never a form of defaults: the General page auto-saves, so a
// form filled from a failed config read would be written back over the user's
// real settings.
import { readOutcome, type ApiResult } from '$lib/api.js';
import type { AssistantStatus, InstalledItem } from '$lib/types.js';
import type { PreferencesData } from '$lib/setup/contracts.js';
import { toPreferencesForm } from './preferences-form';

export type AssistantRow = { family: string; name: string; version?: string; configured: boolean };
export type ExtensionRow = { name: string; kind: string; enabled: boolean };

/** The saved preferences, with an empty display name filled from the system
 *  user name (`guess`) — the default the setup wizard already applies.
 *  `prefilled` says the name was guessed, not read, so the page can save it. */
export async function readPreferences(
  api: { tryGetConfig: () => Promise<ApiResult<Record<string, string>>> },
  guess: () => Promise<string> = async () => '',
): Promise<{ prefs: PreferencesData | null; error: string | null; prefilled: boolean }> {
  const { data, error } = readOutcome(await api.tryGetConfig());
  if (!data) return { prefs: null, error, prefilled: false };
  const prefs = toPreferencesForm(data);
  if (prefs.displayName.trim()) return { prefs, error, prefilled: false };
  const name = (await guess()).trim();
  return name ? { prefs: { ...prefs, displayName: name }, error, prefilled: true } : { prefs, error, prefilled: false };
}

export async function readAssistants(api: {
  tryDetectAssistants: () => Promise<ApiResult<Array<AssistantStatus & { version?: string }>>>;
}): Promise<{ assistants: AssistantRow[]; error: string | null }> {
  const { data, error } = readOutcome(await api.tryDetectAssistants());
  const assistants = (data ?? []).map((a) => ({
    family: a.family,
    name: a.name,
    version: a.version,
    configured: a.configured,
  }));
  return { assistants, error };
}

export async function readExtensions(api: {
  tryGetInstalledItems: () => Promise<ApiResult<Array<Partial<InstalledItem>>>>;
}): Promise<{ extensions: ExtensionRow[]; error: string | null }> {
  const { data, error } = readOutcome(await api.tryGetInstalledItems());
  const extensions = (data ?? []).map((i) => ({ name: i.name ?? '', kind: i.kind ?? 'unknown', enabled: i.enabled ?? true }));
  return { extensions, error };
}
