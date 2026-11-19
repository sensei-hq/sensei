// The Settings pages' reads, each the daemon's answer or why it failed (#288).
// A failed read is shown as one — never an empty list ("none detected", "none
// installed") and never a form of defaults: the General page auto-saves, so a
// form filled from a failed config read would be written back over the user's
// real settings.
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';
import type { AssistantInstall, InstalledItem } from '$lib/types.js';
import type { PreferencesData } from '$lib/setup/contracts.js';
import { toPreferencesForm } from './preferences-form';

export type AssistantRow = {
  family: string;
  name: string;
  kind: AssistantInstall['kind'];
  version: string | null;
  installed: boolean;
  configured: boolean;
  /** When detection last ran for it (#455). */
  detectedAt: string;
};
export type ExtensionRow = { name: string; kind: string; enabled: boolean | null };
/** A row the daemon sent that cannot be listed: an item needs a name and a
 *  kind to be drawn and toggled, and neither is invented (D83). */
export type SkippedExtension = { name: string; reason: string };

/** The saved preferences, with an empty display name filled from the system
 *  user name (`guess`) — the default the setup wizard already applies.
 *  `prefilled` says the name was guessed, not read, so the page can save it. */
export async function readPreferences(
  api: { tryGetConfig: () => Promise<ApiResult<Record<string, string>>> },
  guess: () => Promise<string> = async () => '',
): Promise<{ prefs: PreferencesData | null; error: string | null; prefilled: boolean }> {
  const { data, error } = readOutcome(await api.tryGetConfig());
  if (!data) return { prefs: null, error, prefilled: false };
  let prefs: PreferencesData;
  try {
    prefs = toPreferencesForm(data);
  } catch (e) {
    return { prefs: null, error: `The stored preferences cannot be read: ${(e as Error).message}`, prefilled: false };
  }
  if (prefs.displayName.trim()) return { prefs, error, prefilled: false };
  const name = (await guess()).trim();
  return name ? { prefs: { ...prefs, displayName: name }, error, prefilled: true } : { prefs, error, prefilled: false };
}

/** This machine's installs, as detection stored them (#455), or why they could not be read. */
export async function readAssistants(api: {
  tryGetAssistantInstalls: () => Promise<ApiResult<AssistantInstall[]>>;
}): Promise<{ assistants: AssistantRow[]; error: string | null }> {
  const { data, error } = readOutcome(await api.tryGetAssistantInstalls());
  const assistants = (data ?? []).map((a) => ({
    family: a.id,
    name: a.name,
    kind: a.kind,
    version: a.version,
    installed: a.installed,
    configured: a.configured,
    detectedAt: a.detected_at,
  }));
  return { assistants, error };
}

/** "Detect again": probe and store. Null when stored, else why not. */
export async function detectAgain(api: {
  tryDetectAssistantInstalls: () => Promise<ApiResult<AssistantInstall[]>>;
}): Promise<string | null> {
  const answer = await api.tryDetectAssistantInstalls();
  return answer.ok ? null : failureReason(answer.error);
}

/** The installed items, each as the daemon described it. Three fields stay
 *  unknown rather than defaulted (D83): a row with no name or no kind is not
 *  listed (it cannot be drawn or toggled, and `kind ?? 'unknown'` would build
 *  a toggle URL for a kind that does not exist), and `enabled` is `null` when
 *  the daemon did not say — never `true`, which would draw an extension as on.
 *  The input stays `Partial<>`: the guard is the runtime behaviour, and an
 *  older daemon is exactly the case it covers. */
export async function readExtensions(api: {
  tryGetInstalledItems: () => Promise<ApiResult<Array<Partial<InstalledItem>>>>;
}): Promise<{ extensions: ExtensionRow[]; skipped?: SkippedExtension[]; error: string | null }> {
  const { data, error } = readOutcome(await api.tryGetInstalledItems());
  const extensions: ExtensionRow[] = [];
  const skipped: SkippedExtension[] = [];
  if (!data) return { extensions, error };
  for (const i of data) {
    const name = i.name ?? '';
    const kind = i.kind ?? '';
    if (!name || !kind) {
      skipped.push({ name, reason: `${!name ? 'name' : ''}${!name && !kind ? ' and ' : ''}${!kind ? 'kind' : ''} not reported` });
      continue;
    }
    extensions.push({ name, kind, enabled: i.enabled ?? null });
  }
  // `skipped` is present only when something was skipped: a read that listed
  // every row says nothing about rows it never had.
  return skipped.length > 0 ? { extensions, skipped, error } : { extensions, error };
}
