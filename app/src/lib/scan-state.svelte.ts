/**
 * Scan state classes — project state + activity state for the scan page.
 */

import { ReactiveStageContext } from './stage.svelte.js';
import type { ScanProject, ScanProjectFolder, ScanFolderEvent, ActivityEvent, StateEvent } from './types.js';

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Compute common parent path from an array of absolute paths. */
export function commonParent(paths: string[]): string {
  if (paths.length === 0) return '';
  if (paths.length === 1) return paths[0].substring(0, paths[0].lastIndexOf('/'));

  const parts = paths.map(p => p.split('/'));
  const common: string[] = [];

  for (let i = 0; i < parts[0].length; i++) {
    const segment = parts[0][i];
    if (parts.every(p => p[i] === segment)) {
      common.push(segment);
    } else {
      break;
    }
  }

  return common.join('/') || '/';
}

// ── ScanProjectState ─────────────────────────────────────────────────────────

export class ScanProjectState extends ReactiveStageContext<ScanProject> {

  /**
   * Override update to merge folders by id rather than replacing the whole array.
   * Daemon sends partial folder updates — only changed folders in the array.
   */
  override update(data: ScanProject | Partial<ScanProject> | (ScanProject | Partial<ScanProject>)[]) {
    const patches = Array.isArray(data) ? data : [data];
    let changed = false;

    for (const patch of patches) {
      if (!patch.id) continue;
      const idx = this.items.findIndex(p => p.id === patch.id);
      if (idx < 0) continue;

      const existing = this.items[idx];
      const merged = { ...existing, ...patch };

      // Merge folders by id if patch contains folders
      if (patch.folders && existing.folders) {
        const folderMap = new Map(existing.folders.map(f => [f.id, f]));
        for (const pf of patch.folders) {
          folderMap.set(pf.id, { ...(folderMap.get(pf.id) ?? pf as ScanProjectFolder), ...pf });
        }
        merged.folders = Array.from(folderMap.values());
      }

      this.items[idx] = merged as ScanProject;
      changed = true;
    }

    if (changed) this.items = [...this.items];
  }

  /**
   * Apply a folder-entity SSE event. The daemon sends folder events with a
   * projectId field for routing. On add: find the project and add/merge the
   * folder, or create a placeholder project if the project isn't in state yet
   * (happens on re-scans where project_add is not re-emitted). On update:
   * delegate to the existing folder-merge logic in update().
   */
  applyFolder(event: StateEvent<ScanFolderEvent>) {
    const folders = Array.isArray(event.data) ? event.data : [event.data];
    for (const folder of folders) {
      const { projectId, ...folderData } = folder;
      const proj = this.items.find(p => p.id === projectId);

      if (event.action === 'add') {
        if (proj) {
          if (!proj.folders.find(f => f.id === folder.id)) {
            this.items = this.items.map(p =>
              p.id === projectId ? { ...p, folders: [...p.folders, folderData] } : p
            );
          }
        } else {
          // Project not yet in state (re-scan of existing project) — create placeholder
          this.add({
            id: projectId,
            name: folder.name,
            status: 'scanning',
            folders: [folderData],
            autoDetected: true,
            confidence: 'high',
          } as ScanProject);
        }
      } else if (event.action === 'update') {
        // Reuse the folder-merge logic already in update()
        this.update({ id: projectId, folders: [folderData] } as Partial<ScanProject>);
      }
    }
  }

  // ── Derived ──────────────────────────────────────────────

  /** Derive project display path from its folders. */
  projectPath(project: ScanProject): string {
    return commonParent(project.folders.map(f => f.path));
  }

  get totalFolders() {
    return this.items.reduce((s, p) => s + p.folders.length, 0);
  }

  get readyFolders() {
    return this.items.reduce((s, p) => s + p.folders.filter(f => f.status === 'indexed').length, 0);
  }

  get totalFiles() {
    return this.items.reduce((s, p) => s + p.folders.reduce((fs, f) => fs + f.filesTotal, 0), 0);
  }

  get completedFiles() {
    return this.items.reduce((s, p) => s + p.folders.reduce((fs, f) => fs + f.filesCompleted, 0), 0);
  }

  get scanning() {
    return this.items.some(p => p.status === 'scanning' || p.status === 'indexing');
  }

  get done() {
    return this.items.length > 0 && this.items.every(p => p.status === 'active' || p.status === 'failed');
  }
}

// ── ScanActivityState ────────────────────────────────────────────────────────

export class ScanActivityState extends ReactiveStageContext<ActivityEvent> {

  /** Recent events, newest first. */
  get recent() {
    return this.items.slice(-100).reverse();
  }

  /** Total elapsed time from latest event. */
  get totalElapsed() {
    return this.items.length > 0 ? this.items[this.items.length - 1].elapsed : 0;
  }

  /** Stats bar counts derived from activity levels + messages. */
  get discovered() {
    return this.items.filter(e => e.level === 'discover' && e.message.includes('found')).length;
  }

  get queued() {
    return this.items.filter(e => e.level === 'queue').length;
  }

  get processed() {
    return this.items.filter(e => e.level === 'process' && e.message.includes('graph extracted')).length;
  }

  get scanComplete() {
    return this.items.some(e => e.level === 'success');
  }
}
