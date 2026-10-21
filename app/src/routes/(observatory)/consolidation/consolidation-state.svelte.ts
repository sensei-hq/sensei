// Observatory · Consolidation — the screen's actions: run a consolidation
// (propose a merged ruleset) and approve one. One action runs at a time, and
// `notice` says why the last one was skipped or failed. The API and the reload
// are handed in, so a spec can fail them.
import { failureReason, type ApiResult } from '$lib/api.js';
import type { ConsolidateResult } from './consolidation-view.js';

export interface ConsolidationApi {
  consolidateRules: () => Promise<ApiResult<ConsolidateResult>>;
  approveConsolidatedRuleset: (id: string) => Promise<ApiResult<unknown>>;
}

export type Reload = () => Promise<unknown>;

export class ConsolidationState {
  /** The running action — the buttons show activity and cannot double-fire. */
  busy = $state<'consolidate' | 'approve' | null>(null);
  /** Why the last action was skipped or failed. */
  notice = $state<string | null>(null);

  async consolidate(api: ConsolidationApi, reload: Reload): Promise<void> {
    this.busy = 'consolidate';
    this.notice = null;
    const res = await api.consolidateRules();
    this.busy = null;
    if (!res.ok) {
      this.notice = `Consolidation failed: ${failureReason(res.error)}`;
      return;
    }
    if ('skipped' in res.data && res.data.skipped) {
      this.notice = res.data.reason;
      return;
    }
    await reload();
  }

  async approve(api: ConsolidationApi, id: string, reload: Reload): Promise<void> {
    this.busy = 'approve';
    this.notice = null;
    const res = await api.approveConsolidatedRuleset(id);
    this.busy = null;
    if (!res.ok) {
      this.notice = `Approval failed: ${failureReason(res.error)}`;
      return;
    }
    await reload();
  }
}
