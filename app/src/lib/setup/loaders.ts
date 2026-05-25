/**
 * Wizard data loaders — fetch all wizard data from daemon in parallel.
 *
 * Config table uses jsonb — values are native JSON objects, not strings.
 */

import { senseiApi } from '$lib/api.js';
import type { AssistantFamily } from '$lib/types.js';
import type { DaemonAssistantFamily, PreferencesData, WizardLoadData } from './contracts.js';

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

/** Fetch all wizard data from daemon in parallel. */
export async function loadWizardData(port: number): Promise<WizardLoadData> {
  const api = senseiApi(port);
  const [config, families, roots, projects, libs] = await Promise.all([
    api.getConfig(),
    api.detectAssistantFamilies(),
    api.getScanRoots(),
    api.listProjects(),
    api.getLibs(),
  ]);

  return {
    completion: extractCompletion(config),
    preferences: extractPreferences(config),
    assistantFamilies: mapFamilies(families),
    roots: roots as any[],
    projects: projects as any[],
    libraries: libs as any,
    mcps: [],
  };
}
