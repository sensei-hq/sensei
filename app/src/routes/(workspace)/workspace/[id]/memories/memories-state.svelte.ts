/**
 * Reactive state for the Workspace window · Memories screen (ready-to-share lane).
 *
 * Owns every derivation and status-driven copy the screen needs: the
 * ready-to-share hero counts + copy, the generalised chip semantics, the
 * per-memory generalise loading/error state machine, and the widen-scope
 * submenu (scope ladder + governance gating). Extracted into a class so it can
 * be unit-tested with a `*-state.spec.svelte.ts`.
 *
 * It also runs the screen's actions — propose and decide a share batch,
 * generalise, widen — with the API and the reload handed in, so the whole thing
 * is testable without a transport. Every failure is shown, never swallowed.
 */
import { SvelteSet, SvelteMap } from 'svelte/reactivity';
import { failureReason, type ApiResult } from '$lib/api.js';
import type { WorkspaceMemory } from '$lib/types.js';

/** The writes the screen makes — injected so a spec can fail them. */
export interface MemoriesApi {
    tryCreateMemoryShareBatch: (workspaceId: string, memoryIds: string[]) => Promise<ApiResult<{ id: string }>>;
    tryDecideMemoryShareBatch: (
        workspaceId: string,
        batchId: string,
        status: BatchDecision,
    ) => Promise<ApiResult<void>>;
    generaliseMemory: (id: string) => Promise<ApiResult<unknown>>;
    promoteMemory: (id: string, body: { gov_scope: string }) => Promise<ApiResult<unknown>>;
}

export type BatchDecision = 'approved' | 'rejected' | 'withdrawn';

/** Re-read the screen's load after a write (the page passes invalidateAll). */
export type Reload = () => Promise<unknown>;

/** What a failed generalise means to the user, by status. */
export function generaliseMessage(status: number): string {
    if (status === 503) return "couldn't generalise this right now — the model was unavailable. try again.";
    if (status === 422) return 'this memory has no content to generalise.';
    return "couldn't generalise this right now. try again.";
}

/** What a refused widen means to the user, by status. */
export function widenMessage(status: number): string {
    if (status === 409) return "this memory isn't promotable yet.";
    return "couldn't widen this right now. try again.";
}

/** Semantic chip tone — the component maps this to utility classes. */
export type ChipTone = 'success' | 'neutral' | 'accent';

export interface GeneralisedChipView {
    label: string;
    tone: ChipTone;
    done: boolean;
}

/**
 * A rung on the scope-widening ladder. `governed` rungs enter a maintainer
 * triage queue (see docs/spec/pipeline/memory promotion ladder:
 * workspace → user → org → collective), so the UI must surface the approval note
 * and a confirm step before promoting — nothing widens beyond the user's own
 * scope silently. `scope` is the `gov_scope` value handed to `promoteMemory`.
 */
export interface ScopeRung {
    scope: string;
    label: string;
    governed: boolean;
}

/**
 * workspace → user is the user's own scope (no governance). Anything wider enters
 * the collective's triage queue. Mirrors the promotion ladder in
 * docs/spec/pipeline/memory.md.
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

export class WorkspaceMemoriesState {
    memories = $state<WorkspaceMemory[]>([]);

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

    constructor(memories: WorkspaceMemory[] = []) {
        if (memories.length > 0) this.setMemories(memories);
    }

    setMemories(memories: WorkspaceMemory[]): void {
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
                    ? 'one workspace memory generalised cleanly.'
                    : `${n} workspace memories generalised cleanly.`,
            body:
                'sensei has rewritten '
                + (n === 1 ? 'it' : 'them')
                + ' stack-agnostic, ready to widen up the scope ladder. '
                + 'review the batch before anything leaves this workspace.',
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
    chipFor(m: WorkspaceMemory): GeneralisedChipView {
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

    /** Only workspace-scope memories that have passed generalisation can widen. */
    canWiden(m: WorkspaceMemory): boolean {
        return m.generalised === true && m.scope === 'workspace';
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

    // ── Share batch ──────────────────────────────────────────────────────────

    /** Memory ids ticked for the next share batch. */
    readonly selected = new SvelteSet<string>();
    creating = $state(false);
    /** Why the last propose failed; the selection is kept so it can be retried. */
    batchError = $state<string | null>(null);
    #deciding = new SvelteSet<string>();
    #decideErrors = new SvelteMap<string, string>();
    /** The row whose What / Because / Consequence drawer is open. */
    openedMemoryId = $state<string | null>(null);

    readonly openedMemory = $derived(
        this.openedMemoryId ? this.memories.find((m) => m.id === this.openedMemoryId) ?? null : null,
    );

    toggleSelected(id: string): void {
        if (this.selected.has(id)) this.selected.delete(id);
        else this.selected.add(id);
    }

    /** Stage every generalised (ready) memory for the next batch. */
    reviewNextBatch(): void {
        this.selected.clear();
        for (const m of this.ready) this.selected.add(m.id);
    }

    async proposeBatch(api: MemoriesApi, workspaceId: string, reload: Reload): Promise<void> {
        if (this.selected.size === 0 || !workspaceId) return;
        this.creating = true;
        this.batchError = null;
        const res = await api.tryCreateMemoryShareBatch(workspaceId, Array.from(this.selected));
        if (res.ok) {
            this.selected.clear();
            await reload();
        } else {
            this.batchError = `The batch was not proposed: ${failureReason(res.error)}`;
        }
        this.creating = false;
    }

    isDeciding(batchId: string): boolean {
        return this.#deciding.has(batchId);
    }

    decideError(batchId: string): string | null {
        return this.#decideErrors.get(batchId) ?? null;
    }

    async decide(api: MemoriesApi, workspaceId: string, batchId: string, status: BatchDecision, reload: Reload): Promise<void> {
        if (!workspaceId) return;
        this.#deciding.add(batchId);
        this.#decideErrors.delete(batchId);
        const res = await api.tryDecideMemoryShareBatch(workspaceId, batchId, status);
        if (res.ok) await reload();
        else this.#decideErrors.set(batchId, `Not ${status}: ${failureReason(res.error)}`);
        this.#deciding.delete(batchId);
    }

    // ── Generalise / widen actions ───────────────────────────────────────────

    /** Rewrite a memory stack-agnostic (the model takes 1–4s). On success the
     *  reload flips its chip; on failure the original stays and the row says why. */
    async generalise(api: MemoriesApi, id: string, reload: Reload): Promise<void> {
        this.startGeneralise(id);
        const res = await api.generaliseMemory(id);
        if (res.ok) {
            await reload();
            this.finishGeneralise(id);
        } else {
            this.failGeneralise(id, generaliseMessage(res.error.status));
        }
    }

    /** Pick a rung: the user's own scope widens at once; a governed one waits
     *  for `confirmWiden`. */
    async pickScope(api: MemoriesApi, id: string, rung: ScopeRung, reload: Reload): Promise<void> {
        if (this.chooseScope(rung) === 'proceed') await this.#widen(api, id, rung.scope, reload);
    }

    async confirmWiden(api: MemoriesApi, id: string, reload: Reload): Promise<void> {
        if (this.pendingScope) await this.#widen(api, id, this.pendingScope, reload);
    }

    async #widen(api: MemoriesApi, id: string, scope: string, reload: Reload): Promise<void> {
        this.startPromote(id);
        const res = await api.promoteMemory(id, { gov_scope: scope });
        if (res.ok) {
            this.finishPromote(id);
            await reload();
        } else {
            this.failPromote(id, widenMessage(res.error.status));
        }
    }
}
