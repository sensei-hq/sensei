import { SvelteSet, SvelteMap } from 'svelte/reactivity';
import { failureReason, senseiApi } from './api.js';
import { appState } from './appstate.svelte.js';
import type { Memory, MemoryDetail, MemoryStatus } from './setup/contracts.js';

type Tab = 'triage' | 'active' | 'archive';

/** A memory in a terminal lifecycle state has no further transitions — the
 *  daemon returns 409 on challenge/dismiss. The UI disables those actions. */
export function isTerminalStatus(status: MemoryStatus): boolean {
    return status === 'archived' || status === 'rejected';
}

/** The four active-set lifecycle verbs the triage screen can apply. */
export type MemoryAction = 'reinforce' | 'challenge' | 'archive' | 'dismiss';

/**
 * Deterministic, non-fatal message for a failed lifecycle action. 409 means the
 * memory is already terminal (challenge/dismiss only); everything else is a
 * transient "try again". Voice: lowercase, no exclamation. These are
 * status-driven, so they never route through narration-cache.
 */
export function actionErrorMessage(action: MemoryAction, status: number): string {
    if (status === 409) return 'this memory is already archived or rejected.';
    return `couldn't ${action} this right now. try again.`;
}

/** The formatted 7-day usage strip for the memory-detail stage. */
export interface MemoryUsageStrip {
    loaded:   string;
    followed: string;
    skipped:  string;
    window:   string;
}

/** The strength scale the daemon keeps: memories start at 1 and an outcome
 *  adds 0.5, capped at 5 (`memory_outcome_apply`). */
export const STRENGTH_MAX = 5;

/** A memory's strength as every screen shows it (audit 2026-10-28, item 1:
 *  it read "300%", a full bar and "3.0 / 5" on three screens). */
export interface StrengthView {
  label: string;
  /** 0–1, for a bar. */
  fraction: number;
  /** The bar's CSS width. */
  width: string;
  full: boolean;
  /** Five dots, filled to the rounded strength. */
  dots: string;
}

export function strengthView(strength: number | null | undefined): StrengthView {
  // Not reported is not zero: a made-up "0.0 / 5" would read as a weak memory.
  if (strength === null || strength === undefined) {
    return { label: '—', fraction: 0, width: '0%', full: false, dots: '' };
  }
  const s = Math.max(0, Math.min(STRENGTH_MAX, strength));
  const filled = Math.round(s);
  return {
    label: `${s.toFixed(1)} / ${STRENGTH_MAX}`,
    fraction: s / STRENGTH_MAX,
    width: `${Math.round((s / STRENGTH_MAX) * 100)}%`,
    full: s >= STRENGTH_MAX,
    dots: '\u25CF'.repeat(filled) + '\u25CB'.repeat(STRENGTH_MAX - filled)
  };
}

/**
 * Format the 7-day memory-usage telemetry into the detail "usage" strip.
 * The numbers are deterministic; only the pluralization/labels live here so
 * the component stays a pure template. Genuinely-zero telemetry renders as
 * zeros ("loaded 0 times · followed 0 · skipped 0"), never blank.
 */
export function memoryUsageStrip(u: {
    loaded_last_7d:   number;
    followed_last_7d: number;
    skipped_last_7d:  number;
}): MemoryUsageStrip {
    const loaded = u.loaded_last_7d;
    return {
        loaded:   `loaded ${loaded} ${loaded === 1 ? 'time' : 'times'}`,
        followed: `followed ${u.followed_last_7d}`,
        skipped:  `skipped ${u.skipped_last_7d}`,
        window:   'in the last 7 days',
    };
}

const STATUSES_BY_TAB: Record<Tab, MemoryStatus[]> = {
    triage:  ['proposed'],
    active:  ['active', 'reinforced', 'challenged', 'battle_tested'],
    archive: ['archived', 'rejected'],
};

class MemoryState {
    /** A memory's strength as Learnings shows it — the one shared view. */
    strengthOf(m: { strength: number | null | undefined }): StrengthView {
        return strengthView(m.strength);
    }

    triage   = $state<Memory[]>([]);
    active   = $state<Memory[]>([]);
    archive  = $state<Memory[]>([]);
    detail   = $state<MemoryDetail | null>(null);
    selected = $state<string | null>(null);
    loading  = $state(false);
    /** Why the lists could not be read or refreshed, or null (#288). A failed
     *  read used to fold into an empty tab. */
    listError = $state<string | null>(null);

    // Per-memory lifecycle-action UI: ids with an in-flight request, and the
    // last error message keyed by id. The error persists until the next attempt
    // so a 409/transient failure stays visible instead of flashing away.
    #acting = new SvelteSet<string>();
    #actionErrors = new SvelteMap<string, string>();

    // The workspace scope the lists were loaded under (undefined = global /
    // observatory view). Re-fetches after a mutation reuse it so a scoped view
    // never pulls in memories from other workspaces.
    #workspaceId: string | undefined = undefined;

    triageCount  = $derived(this.triage.length);
    activeCount  = $derived(this.active.length);
    archiveCount = $derived(this.archive.length);

    async load(workspaceId?: string) {
        this.#workspaceId = workspaceId;
        this.loading = true;
        const [t, a, x] = await Promise.all([
            this.fetchTab('triage',  workspaceId),
            this.fetchTab('active',  workspaceId),
            this.fetchTab('archive', workspaceId),
        ]);
        // Any tab unread means the lists are unknown — none is shown.
        this.listError = t.error ?? a.error ?? x.error;
        this.triage  = this.listError ? [] : t.memories;
        this.active  = this.listError ? [] : a.memories;
        this.archive = this.listError ? [] : x.memories;
        this.loading = false;
    }

    private async fetchTab(tab: Tab, workspaceId?: string): Promise<{ memories: Memory[]; error: string | null }> {
        const api = senseiApi(appState.port);
        const statuses = STATUSES_BY_TAB[tab];
        const buckets = await Promise.all(statuses.map(status =>
            api.tryListMemories({ status, workspace_id: workspaceId, limit: 500 })
        ));
        const failed = buckets.find((b) => !b.ok);
        if (failed && !failed.ok) return { memories: [], error: failureReason(failed.error) };
        return { memories: buckets.flatMap((b) => (b.ok ? b.data.memories : [])), error: null };
    }

    /** Re-read one tab after an action. A failed read keeps the tab as it was
     *  and says the lists could not be refreshed. */
    private async refreshTab(tab: Tab, current: Memory[]): Promise<Memory[]> {
        const read = await this.fetchTab(tab, this.#workspaceId);
        this.listError = read.error;
        return read.error ? current : read.memories;
    }

    async select(id: string) {
        this.selected = id;
        const api = senseiApi(appState.port);
        const res = await api.getMemoryDetail(id);
        if (res.ok) this.detail = res.data;
    }

    async accept(id: string) {
        const api = senseiApi(appState.port);
        const res = await api.acceptProposal(id);
        if (!res.ok) return;
        this.triage = this.triage.filter(m => m.id !== id);
        this.active = await this.refreshTab('active', this.active);
    }

    async reject(id: string) {
        const api = senseiApi(appState.port);
        const res = await api.rejectProposal(id);
        if (!res.ok) return;
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.refreshTab('archive', this.archive);
    }

    // ── Active-set lifecycle actions ─────────────────────────────────────────
    // Each mirrors accept/reject: call the api, bail on failure (surfacing the
    // error), then move the memory to the tab its new status belongs to by
    // re-fetching the destination list. Guarded against double-submit.

    isActing(id: string): boolean {
        return this.#acting.has(id);
    }

    actionError(id: string): string | null {
        return this.#actionErrors.get(id) ?? null;
    }

    #begin(id: string): boolean {
        if (this.#acting.has(id)) return false;
        this.#actionErrors.delete(id);
        this.#acting.add(id);
        return true;
    }

    #fail(id: string, action: MemoryAction, status: number): void {
        this.#acting.delete(id);
        this.#actionErrors.set(id, actionErrorMessage(action, status));
    }

    #done(id: string): void {
        this.#acting.delete(id);
        this.#actionErrors.delete(id);
    }

    /** Strengthen a proven memory. Stays in the active set — re-fetch active so
     *  the bumped strength is reflected. */
    async reinforce(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).reinforceMemory(id);
        if (!res.ok) { this.#fail(id, 'reinforce', res.error.status); return; }
        this.active = await this.refreshTab('active', this.active);
        this.#done(id);
    }

    /** Contest a memory. `challenged` is still in the active set, so the row
     *  stays put — re-fetch active to reflect the new status. 409 = terminal. */
    async challenge(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).challengeMemory(id);
        if (!res.ok) { this.#fail(id, 'challenge', res.error.status); return; }
        this.active = await this.refreshTab('active', this.active);
        this.#done(id);
    }

    /** Retire a memory. Leaves the active/triage set for the archive tab. */
    async archiveMemory(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).archiveMemory(id);
        if (!res.ok) { this.#fail(id, 'archive', res.error.status); return; }
        this.active = this.active.filter(m => m.id !== id);
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.refreshTab('archive', this.archive);
        this.#done(id);
    }

    /** Reject a memory outright. `rejected` lives in the archive tab. 409 =
     *  already terminal. */
    async dismiss(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).dismissMemory(id);
        if (!res.ok) { this.#fail(id, 'dismiss', res.error.status); return; }
        this.active = this.active.filter(m => m.id !== id);
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.refreshTab('archive', this.archive);
        this.#done(id);
    }
}

export const memoryState = new MemoryState();
