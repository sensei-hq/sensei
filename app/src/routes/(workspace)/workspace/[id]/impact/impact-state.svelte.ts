// Workspace · Impact — the screen's state: the picked verdict, the impact-log
// form, and the two writes (log an impact, decide one), each of which says why
// it failed rather than carrying on (#304). The API and the reload are handed
// in, so a spec can fail them.
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { failureReason, type ApiResult } from '$lib/api.js';
import type { ImpactBuckets, ImpactRow } from '$lib/impact.js';
import type { ImpactVerdictEntry } from '$lib/types.js';

export type ImpactOutcome = 'success' | 'mixed' | 'failure';

export interface WorkspaceImpactApi {
  tryCreateImpactVerdict: (workspaceId: string, title: string, note?: string) => Promise<ApiResult<{ id: string }>>;
  tryDecideImpactVerdict: (workspaceId: string, verdictId: string, outcome: ImpactOutcome) => Promise<ApiResult<void>>;
}

export type Reload = () => Promise<unknown>;

/** Measured verdicts, loudest to act on first: negative, positive, neutral, pending. */
export function flattenBuckets(buckets: ImpactBuckets | null): ImpactRow[] {
  if (!buckets) return [];
  return [...buckets.negative, ...buckets.positive, ...buckets.neutral, ...buckets.pending];
}

/** The user-logged impacts still awaiting a verdict, and those decided. */
export function splitLog(log: ImpactVerdictEntry[]): { pending: ImpactVerdictEntry[]; decided: ImpactVerdictEntry[] } {
  return {
    pending: log.filter((e) => e.verdict === 'pending'),
    decided: log.filter((e) => e.verdict !== 'pending'),
  };
}

export class WorkspaceImpactState {
  /** The picked row; null until the user clicks. */
  selectedId = $state<string | null>(null);
  logForm = $state({ title: '', note: '' });
  logging = $state(false);
  /** Why the last log failed; what was typed is kept. */
  logError = $state<string | null>(null);
  #deciding = new SvelteSet<string>();
  #decideErrors = new SvelteMap<string, string>();

  /** The picked row, or the first when none is picked or it left the list. */
  selectedIn(rows: ImpactRow[]): ImpactRow | undefined {
    return rows.find((r) => r.id === this.selectedId) ?? rows[0];
  }

  async logImpact(api: WorkspaceImpactApi, workspaceId: string, reload: Reload): Promise<void> {
    const title = this.logForm.title.trim();
    if (!workspaceId || !title) return;
    this.logging = true;
    this.logError = null;
    const res = await api.tryCreateImpactVerdict(workspaceId, title, this.logForm.note.trim() || undefined);
    if (res.ok) {
      this.logForm = { title: '', note: '' };
      await reload();
    } else {
      this.logError = `The impact was not logged: ${failureReason(res.error)}`;
    }
    this.logging = false;
  }

  isDeciding(id: string): boolean {
    return this.#deciding.has(id);
  }

  decideError(id: string): string | null {
    return this.#decideErrors.get(id) ?? null;
  }

  async decide(api: WorkspaceImpactApi, workspaceId: string, id: string, outcome: ImpactOutcome, reload: Reload): Promise<void> {
    if (!workspaceId) return;
    this.#deciding.add(id);
    this.#decideErrors.delete(id);
    const res = await api.tryDecideImpactVerdict(workspaceId, id, outcome);
    if (res.ok) await reload();
    else this.#decideErrors.set(id, `Not recorded: ${failureReason(res.error)}`);
    this.#deciding.delete(id);
  }
}
