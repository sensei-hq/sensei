import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export interface FolderEntry {
  id: string;
  path: string;
  note: string;
  watched: boolean;
}

export async function load() {
  const api = senseiApi(appState.port);
  try {
    const roots = await api.getScanRoots();
    const folders: FolderEntry[] = roots.map((r, i) => ({
      id: `root-${i}`,
      path: r.path,
      note: r.scanned ? `${r.repos_found} folders found` : '',
      watched: r.scanned,
    }));
    return { folders };
  } catch {
    return { folders: [] as FolderEntry[] };
  }
}
