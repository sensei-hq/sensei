/**
 * Reactive state for the Project window · Memories screen (ready-to-share lane).
 *
 * Owns every derivation and status-driven copy the screen needs: the
 * ready-to-share hero counts + copy, the generalised chip semantics, the
 * per-memory generalise loading/error state machine, and the widen-scope
 * submenu (scope ladder + governance gating). Extracted into a class so it can
 * be unit-tested with a `*-state.spec.svelte.ts`.
 *
 * The component keeps the network + navigation (invalidateAll) — this class
 * only holds the view state and the pure transitions around those calls, so
 * the whole thing is testable without a transport.
 */
import { SvelteSet, SvelteMap } from 'svelte/reactivity';
import type { ProjectMemory } from '$lib/types.js';

/** Semantic chip tone — the component maps this to utility classes. */
export type ChipTone = 'success' | 'neutral' | 'accent';

export interface GeneralisedChipView {
    label: string;
    tone: ChipTone;
    done: boolean;
}

/**
 * A rung on the scope-widening ladder. `governed` rungs enter a maintainer
 * triage queue (see docs/llm-spec/pipeline/memory promotion ladder:
 * project → user → org → collective), so the UI must surface the approval note
 * and a confirm step before promoting — nothing widens beyond the user's own
 * scope silently. `scope` is the `gov_scope` value handed to `promoteMemory`.
 */
export interface ScopeRung {
    scope: string;
    label: string;
    governed: boolean;
}

/**
 * project → user is the user's own scope (no governance). Anything wider enters
 * the collective's triage queue. Mirrors the promotion ladder in
 * docs/llm-spec/pipeline/memory.md.
 */
export const SCOPE_LADDER: readonly ScopeRung[] = [
    { scope: 'user', label: 'widen to user', governed: false },
    { scope: 'org', label: 'widen to org', governed: true },
    { scope: 'global', label: 'widen to collective', governed: true },
];

export interface HeroDisplay {
    eyebrow: string;
    headline: string;
    body: string;
    action: string;
    meta: string;
}

/** Result of picking a scope rung — proceed immediately, or wait for the
 *  user to confirm the governance step. */
export type ScopeChoice = 'proceed' | 'confirm';

export class ProjectMemoriesState {
    memories = $state<ProjectMemory[]>([]);

    // Per-memory generalise UI state machine. Ids currently running + the last
    // error message keyed by id. Errors persist until the next attempt so the
    // honest 503 stays visible rather than flashing away.
    #generalising = new SvelteSet<string>();
    #generaliseErrors = new SvelteMap<string, string>();

    // Widen submenu: which row's submenu is open, which governed rung is
    // awaiting confirmation, plus per-memory promote loading + error.
    widenOpenId = $state<string | null>(null);
    pendingScope = $state<string | null>(null);
    #promoting = new SvelteSet<string>();
    #promoteErrors = new SvelteMap<string, string>();

    constructor(memories: ProjectMemory[] = []) {
        if (memories.length > 0) this.setMemories(memories);
    }

    setMemories(memories: ProjectMemory[]): void {
        this.memories = memories;
    }

    // ── Derivations ──────────────────────────────────────────────────────────

    /** Memories sensei has rewritten stack-agnostic — the ready-to-share set. */
    readonly ready = $derived(this.memories.filter((m) => m.generalised === true));

    /** Hero "ready" count — the number of generalised memories. Single source of
     *  truth so the hero number and the row chips can never disagree. */
    readonly readyCount = $derived(this.ready.length);

    /** Active memories not yet generalised — still forming below the bar. */
    readonly formingCount = $derived(this.memories.length - this.readyCount);

    readonly hasReady = $derived(this.readyCount > 0);

    /** Ready-to-share hero copy. Sentence case, no exclamations, lowercase
     *  sensei, specific numbers. Only rendered when `hasReady`. */
    readonly heroDisplay: HeroDisplay = $derived.by(() => {
        const n = this.readyCount;
        const forming = this.formingCount;
        return {
            eyebrow: `${n} ready to share`,
            headline:
                n === 1
                    ? 'one project memory generalised cleanly.'
                    : `${n} project memories generalised cleanly.`,
            body:
                'sensei has rewritten '
                + (n === 1 ? 'it' : 'them')
                + ' stack-agnostic, ready to widen up the scope ladder. '
                + 'review the batch before anything leaves this project.',
            action: 'review next batch',
            meta:
                forming > 0
                    ? `${forming} still forming`
                    : 'all active memories generalised',
        };
    });

    /** Mini-heading right label — e.g. "3 active · 2 generalised". */
    readonly countLabel = $derived(
        `${this.memories.length} active · ${this.readyCount} generalised`,
    );

    // ── Generalised chip ──────────────────────────────────────────────────────

    /** Semantic chip for a memory: success when generalised, neutral when not.
     *  The component owns the tone → utility-class mapping. */
    chipFor(m: ProjectMemory): GeneralisedChipView {
        const done = m.generalised === true;
        return {
            label: done ? 'generalised: yes' : 'generalised: no',
            tone: done ? 'success' : 'neutral',
            done,
        };
    }

    // ── Generalise state machine ──────────────────────────────────────────────

    isGeneralising(id: string): boolean {
        return this.#generalising.has(id);
    }

    generaliseError(id: string): string | null {
        return this.#generaliseErrors.get(id) ?? null;
    }

    /** Enter the loading state — clears any prior error so the row shows
     *  activity, not stale failure text (the immediate-feedback rule). */
    startGeneralise(id: string): void {
        this.#generaliseErrors.delete(id);
        this.#generalising.add(id);
    }

    /** Honest degrade — surface the 503/error message, drop the loading flag,
     *  keep the original memory untouched. */
    failGeneralise(id: string, message: string): void {
        this.#generalising.delete(id);
        this.#generaliseErrors.set(id, message);
    }

    /** Success — drop the loading flag and any error. The caller re-fetches so
     *  the memory's `generalised` flag (and chip) flips via setMemories. */
    finishGeneralise(id: string): void {
        this.#generalising.delete(id);
        this.#generaliseErrors.delete(id);
    }

    // ── Widen-scope submenu ───────────────────────────────────────────────────

    /** Only project-scope memories that have passed generalisation can widen. */
    canWiden(m: ProjectMemory): boolean {
        return m.generalised === true && m.scope === 'project';
    }

    isWidenOpen(id: string): boolean {
        return this.widenOpenId === id;
    }

    /** Toggle the submenu for a row. Opening a different row (or re-opening)
     *  resets the pending governance confirmation. */
    toggleWiden(id: string): void {
        if (this.widenOpenId === id) {
            this.closeWiden();
        } else {
            this.widenOpenId = id;
            this.pendingScope = null;
        }
    }

    closeWiden(): void {
        this.widenOpenId = null;
        this.pendingScope = null;
    }

    /**
     * Pick a rung. A governed rung (org / collective) must not promote silently
     * — it stages `pendingScope` and returns `'confirm'` so the UI reveals the
     * governance note + a confirm button. The user's own scope returns
     * `'proceed'` for an immediate promote.
     */
    chooseScope(rung: ScopeRung): ScopeChoice {
        if (rung.governed) {
            this.pendingScope = rung.scope;
            return 'confirm';
        }
        this.pendingScope = null;
        return 'proceed';
    }

    /** Governance note shown before a governed widen is confirmed. */
    governanceNote(scope: string): string {
        return (
            `widening to ${scope} enters the collective's triage queue — `
            + 'a maintainer approves before it is shared. nothing leaves silently.'
        );
    }

    isPromoting(id: string): boolean {
        return this.#promoting.has(id);
    }

    promoteError(id: string): string | null {
        return this.#promoteErrors.get(id) ?? null;
    }

    startPromote(id: string): void {
        this.#promoteErrors.delete(id);
        this.#promoting.add(id);
    }

    failPromote(id: string, message: string): void {
        this.#promoting.delete(id);
        this.#promoteErrors.set(id, message);
    }

    /** Success — drop the loading flag and close the submenu. The promote
     *  created a `proposed` copy at the wider scope; the original row is
     *  unchanged, so there is nothing to re-sync here. */
    finishPromote(id: string): void {
        this.#promoting.delete(id);
        this.closeWiden();
    }
}
