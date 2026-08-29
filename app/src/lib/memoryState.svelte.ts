import { SvelteSet, SvelteMap } from 'svelte/reactivity';
import { senseiApi } from './api.js';
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
    triage   = $state<Memory[]>([]);
    active   = $state<Memory[]>([]);
    archive  = $state<Memory[]>([]);
    detail   = $state<MemoryDetail | null>(null);
    selected = $state<string | null>(null);
    loading  = $state(false);

    // Per-memory lifecycle-action UI: ids with an in-flight request, and the
    // last error message keyed by id. The error persists until the next attempt
    // so a 409/transient failure stays visible instead of flashing away.
    #acting = new SvelteSet<string>();
    #actionErrors = new SvelteMap<string, string>();

    // The project scope the lists were loaded under (undefined = global /
    // observatory view). Re-fetches after a mutation reuse it so a scoped view
    // never pulls in memories from other projects.
    #projectId: string | undefined = undefined;

    triageCount  = $derived(this.triage.length);
    activeCount  = $derived(this.active.length);
    archiveCount = $derived(this.archive.length);

    async load(projectId?: string) {
        this.#projectId = projectId;
        this.loading = true;
        try {
            const [t, a, x] = await Promise.all([
                this.fetchTab('triage',  projectId),
                this.fetchTab('active',  projectId),
                this.fetchTab('archive', projectId),
            ]);
            this.triage  = t;
            this.active  = a;
            this.archive = x;
        } finally {
            this.loading = false;
        }
    }

    private async fetchTab(tab: Tab, projectId?: string): Promise<Memory[]> {
        const api = senseiApi(appState.port);
        const statuses = STATUSES_BY_TAB[tab];
        const buckets = await Promise.all(statuses.map(status =>
            api.listMemories({ status, project_id: projectId, limit: 500 })
        ));
        return buckets.flatMap(b => b.memories);
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
        this.active = await this.fetchTab('active', this.#projectId);
    }

    async reject(id: string) {
        const api = senseiApi(appState.port);
        const res = await api.rejectProposal(id);
        if (!res.ok) return;
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.fetchTab('archive', this.#projectId);
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
        this.active = await this.fetchTab('active', this.#projectId);
        this.#done(id);
    }

    /** Contest a memory. `challenged` is still in the active set, so the row
     *  stays put — re-fetch active to reflect the new status. 409 = terminal. */
    async challenge(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).challengeMemory(id);
        if (!res.ok) { this.#fail(id, 'challenge', res.error.status); return; }
        this.active = await this.fetchTab('active', this.#projectId);
        this.#done(id);
    }

    /** Retire a memory. Leaves the active/triage set for the archive tab. */
    async archiveMemory(id: string) {
        if (!this.#begin(id)) return;
        const res = await senseiApi(appState.port).archiveMemory(id);
        if (!res.ok) { this.#fail(id, 'archive', res.error.status); return; }
        this.active = this.active.filter(m => m.id !== id);
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.fetchTab('archive', this.#projectId);
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
        this.archive = await this.fetchTab('archive', this.#projectId);
        this.#done(id);
    }
}

export const memoryState = new MemoryState();
