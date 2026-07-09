import { senseiApi } from './api.js';
import { appState } from './appstate.svelte.js';
import type { Memory, MemoryDetail, MemoryStatus } from './setup/contracts.js';

type Tab = 'triage' | 'active' | 'archive';

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

    triageCount  = $derived(this.triage.length);
    activeCount  = $derived(this.active.length);
    archiveCount = $derived(this.archive.length);

    async load(projectId?: string) {
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
        this.active = await this.fetchTab('active');
    }

    async reject(id: string) {
        const api = senseiApi(appState.port);
        const res = await api.rejectProposal(id);
        if (!res.ok) return;
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.fetchTab('archive');
    }
}

export const memoryState = new MemoryState();
