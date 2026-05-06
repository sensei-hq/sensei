/**
 * Reactive state for the Diagnostic Logs page.
 * Extracted into a class so it can be tested with spec.svelte.ts.
 */
import { SvelteSet } from 'svelte/reactivity';
import type { LogSession } from '$lib/types.js';
import { dateKey } from './helpers.js';

export class LogsPageState {
    sessions: LogSession[] = $state([]);

    /** ID of the currently selected session. */
    selectedId      = $state<string | null>(null);
    /** Which date groups in the sidebar are expanded. */
    expandedDates   = $state(new SvelteSet<string>());
    /** Accordion: at most one trace detail expanded at a time. */
    expandedTraceId = $state<string | null>(null);

    showModal = $state(false);
    addCtx    = $state('');
    copied    = $state(false);

    readonly session = $derived(
        this.sessions.find(s => s.id === this.selectedId) ?? null
    );

    constructor(sessions: LogSession[] = []) {
        if (sessions.length > 0) this.setSessions(sessions);
    }

    /**
     * Load (or reload) the sessions list.
     * Auto-selects the most recent session and expands its date group
     * on first call (when nothing is selected yet).
     */
    setSessions(sessions: LogSession[]): void {
        this.sessions = sessions;
        if (this.selectedId === null && sessions.length > 0) {
            // sessions[0] is the most recent (loader returns newest-first)
            this.selectedId = sessions[0].id;
            this.expandedDates.add(dateKey(sessions[0].started_at));
        }
    }

    selectSession(id: string): void {
        this.selectedId = id;
        this.expandedTraceId = null;
    }

    toggleDate(dk: string): void {
        if (this.expandedDates.has(dk)) this.expandedDates.delete(dk);
        else this.expandedDates.add(dk);
    }

    toggleTrace(id: string): void {
        this.expandedTraceId = this.expandedTraceId === id ? null : id;
    }

    openModal(): void  { this.showModal = true; }
    closeModal(): void { this.showModal = false; }
    setCopied(v: boolean): void { this.copied = v; }
}
