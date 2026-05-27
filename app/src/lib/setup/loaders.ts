/**
 * Wizard data loaders — fetch all wizard data from daemon in parallel.
 *
 * Config table uses jsonb — values are native JSON objects, not strings.
 */

import { senseiApi } from '$lib/api.js';
import type { AssistantFamily, LibEntry } from '$lib/types.js';
import type { DaemonAssistantFamily, DaemonLibEntry, PreferencesData, WizardLoadData } from './contracts.js';

const STAGES = [
  'welcome', 'preferences', 'assistants', 'roots', 'scan',
  'projects', 'libraries', 'instruments', 'inference', 'assignments', 'done',
];

const PREF_DEFAULTS: PreferencesData = {
  displayName: '', contributeLearnings: true, reviewBeforeShare: true,
  shareSchedule: 'weekly-saturday', downloadCollective: 'weekly',
  correctionAggressiveness: 'balanced', digestCadence: 'daily',
  nudgeOnRegression: true, anonymizedTelemetry: false, showWelcome: true,
};

/** Parse setup.* config keys into a completion map. */
export function extractCompletion(config: Record<string, unknown>): Record<string, 'pending' | 'done'> {
  const result: Record<string, 'pending' | 'done'> = {};
  for (const s of STAGES) {
    result[s] = config[`setup.${s}`] === 'done' ? 'done' : 'pending';
  }
  return result;
}

/** Parse the setup.preferences config key into PreferencesData. */
export function extractPreferences(config: Record<string, unknown>): PreferencesData {
  const stored = config['setup.preferences'];
  if (stored && typeof stored === 'object') {
    return { ...PREF_DEFAULTS, ...(stored as Partial<PreferencesData>) };
  }
  return { ...PREF_DEFAULTS };
}

/** Map daemon AssistantFamily[] to app DaemonAssistantFamily[]. */
function mapFamilies(families: AssistantFamily[]): DaemonAssistantFamily[] {
  return families.map(f => ({
    id: f.family,
    name: f.name,
    selected: f.installed,
    variants: f.members.map(m => ({ id: m.id, name: m.name, installed: m.installed, configured: m.configured })),
  }));
}

/**
 * Map raw daemon library entries to DaemonLibEntry, restoring the per-library
 * `enabled` flag from the persisted `setup.libraries` config key. Library
 * identity is the name (daemon doesn't issue ids). New libs default to enabled.
 */
export function mapLibraries(libs: LibEntry[], config: Record<string, unknown>): DaemonLibEntry[] {
  const stored = config['setup.libraries'];
  const wrapped: Set<string> = new Set();
  const disabled: Set<string> = new Set();
  // Daemon's `set_config_handler` stores non-string values via serde's
  // `to_string`, so jsonb-typed values come back as JSON strings. Parse
  // both forms for safety.
  let parsed: { wrapped?: string[]; disabled?: string[] } | null = null;
  if (typeof stored === 'string') {
    try { parsed = JSON.parse(stored); } catch { /* malformed, treat as empty */ }
  } else if (stored && typeof stored === 'object') {
    parsed = stored as { wrapped?: string[]; disabled?: string[] };
  }
  if (parsed) {
    (parsed.wrapped ?? []).forEach(n => wrapped.add(n));
    (parsed.disabled ?? []).forEach(n => disabled.add(n));
  }
  return libs.map(l => ({
    id: l.id || l.name,
    name: l.name,
    ecosystem: l.ecosystem ?? '',
    version: l.version ?? null,
    description: l.description ?? null,
    pageCount: l.pageCount ?? 0,
    repos: l.repos ?? [],
    repoCount: l.repoCount ?? (l.repos?.length ?? 0),
    // Enabled when on the wrapped list, OR a new lib that hasn't been
    // touched (not on disabled list). Default-on so a fresh scan offers
    // everything by default.
    enabled: wrapped.has(l.name) || !disabled.has(l.name),
  }));
}

/** Fetch all wizard data from daemon in parallel. */
export async function loadWizardData(port: number): Promise<WizardLoadData> {
  const api = senseiApi(port);
  const [config, families, roots, projects, libs, instruments] = await Promise.all([
    api.getConfig(),
    api.detectAssistantFamilies(),
    api.getScanRoots(),
    api.listProjects(),
    api.getLibs(),
    api.listInstruments(),
  ]);

  return {
    completion: extractCompletion(config),
    preferences: extractPreferences(config),
    assistantFamilies: mapFamilies(families),
    roots: roots as any[],
    projects: projects as any[],
    libraries: { total: libs.total, libs: mapLibraries(libs.libs, config) },
    mcps: instruments.mcps,
    routers: [],
  };
}
