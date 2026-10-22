/**
 * Intake screen state — owns the freeform → classify → recommend → confirm flow.
 *
 * Depends only on `IntakeApi` (the one `senseiApi` method it calls) so it unit-tests
 * with an injected fake — no network, no appState. Mirrors the state-owns-the-
 * flow pattern of insights-board.svelte.ts.
 */
import type { ApiResult } from '$lib/api.js';
import type { IntakeGuide, PlaybookRecommendation } from '$lib/types.js';

/** The slice of the daemon client this state needs (lets tests inject a fake). */
export interface IntakeApi {
  recommendPlaybook(body: Record<string, unknown>): Promise<ApiResult<PlaybookRecommendation>>;
}

export type IntakePhase = 'describe' | 'loading' | 'recommended' | 'recorded' | 'error';

export class IntakeState {
  guide = $state<IntakeGuide>({ frame: '', axes: [], playbooks: [] });
  chunk = $state('');
  phase = $state<IntakePhase>('describe');
  rec = $state<PlaybookRecommendation | null>(null);
  error = $state('');
  /** The workspace this intake runs in. A playbook run always happens in a workspace
   *  (no global run) — carried on every recommend/confirm so the daemon scopes
   *  trust to this workspace and records the run against it. */
  readonly workspaceId: string;

  constructor(guide: IntakeGuide, workspaceId: string) {
    this.guide = guide;
    this.workspaceId = workspaceId;
  }

  /** Human title for the recommended playbook, from the guide catalog. */
  get playbookTitle(): string {
    const name = this.rec?.playbook ?? '';
    return this.guide.playbooks.find((p) => p.name === name)?.title ?? name;
  }

  /** Recommend leg: classify + recommend without writing a row (preview). */
  async recommend(api: IntakeApi): Promise<void> {
    const chunk = this.chunk.trim();
    if (!chunk) return;
    this.phase = 'loading';
    this.error = '';
    const res = await api.recommendPlaybook({ chunk, preview: true, workspace_id: this.workspaceId });
    if (!res.ok) {
      this.phase = 'error';
      this.error = res.error.message || 'Could not classify the chunk.';
      return;
    }
    this.rec = res.data;
    this.phase = 'recommended';
    // Auto-select-on-trust: skip the manual confirm when the daemon trusts it.
    if (res.data.auto_select) await this.confirm(api);
  }

  /** Confirm leg: record one confirmed run, reusing the classified axes. */
  async confirm(api: IntakeApi): Promise<void> {
    const r = this.rec;
    if (!r) return;
    const res = await api.recommendPlaybook({
      lifecycle: r.lifecycle,
      intent: r.intent,
      risk: r.risk,
      confirm: true,
      workspace_id: this.workspaceId,
    });
    if (!res.ok) {
      this.phase = 'error';
      this.error = res.error.message || 'Could not record the playbook.';
      return;
    }
    this.phase = 'recorded';
  }

  reset(): void {
    this.chunk = '';
    this.rec = null;
    this.error = '';
    this.phase = 'describe';
  }
}
