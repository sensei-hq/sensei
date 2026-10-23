// Workspace · Instruments — the per-workspace service toggle. Each row is in
// flight while its write is out, and a write the daemon refused says why on
// that row instead of the switch snapping back in silence (#304). The API and
// the reload are handed in, so a spec can fail them.
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { failureReason, type ApiResult } from '$lib/api.js';
import type { WorkspaceService } from '$lib/types.js';

export interface WorkspaceServicesApi {
  trySetWorkspaceServiceScope: (workspaceId: string, serviceId: string, enabled: boolean) => Promise<ApiResult<void>>;
}

export type Reload = () => Promise<unknown>;

export class WorkspaceServicesState {
  #toggling = new SvelteSet<string>();
  #errors = new SvelteMap<string, string>();

  isToggling(id: string): boolean {
    return this.#toggling.has(id);
  }

  toggleError(id: string): string | null {
    return this.#errors.get(id) ?? null;
  }

  async toggle(api: WorkspaceServicesApi, workspaceId: string, service: WorkspaceService, reload: Reload): Promise<void> {
    if (!workspaceId || this.#toggling.has(service.id)) return;
    this.#toggling.add(service.id);
    this.#errors.delete(service.id);
    const res = await api.trySetWorkspaceServiceScope(workspaceId, service.id, !service.enabledForWorkspace);
    if (res.ok) await reload();
    else this.#errors.set(service.id, `Not changed: ${failureReason(res.error)}`);
    this.#toggling.delete(service.id);
  }
}
