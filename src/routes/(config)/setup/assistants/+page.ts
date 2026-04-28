/**
 * Load detected AI coding assistants from the daemon API.
 */
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export interface AssistantEntry {
  id: string;
  name: string;
  installed: boolean;
  configPath: string | null;
}

export async function load() {
  const api = senseiApi(appState.port);

  try {
    const families = await api.detectAssistantFamilies();
    const assistants: AssistantEntry[] = families.map(f => ({
      id: f.family,
      name: f.name,
      installed: f.installed,
      configPath: f.config_path ? shortPath(f.config_path) : null,
    }));
    return { assistants };
  } catch {
    return { assistants: [] as AssistantEntry[] };
  }
}

function shortPath(p: string): string {
  return p.replace(/^\/Users\/[^/]+/, '~');
}
