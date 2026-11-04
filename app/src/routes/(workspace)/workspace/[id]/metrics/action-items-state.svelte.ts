// Metric detail · action items — the state behind each item's one action. The
// component renders `flow(id)` and calls `act` / `confirm` / `cancel`; every
// write goes through here.
//
// - propose-rule / review-file: preview first (nothing is written), then the
//   person confirms and the recommendation is materialized. The PREVIEW decides
//   what is confirmed and what is sent (`materializeOptions`), not the item.
// - snooze: the daemon hides the recommendation until it wakes.
// - open-file: the file is shown in Finder.
// A settled item leaves the list (`settle`) and `done` says what happened. A
// failure is that item's error, never a silent no-op, and acting again retries.
import { SvelteMap } from 'svelte/reactivity';
import { failureReason, type RecommendationPreview, type SenseiApi } from '$lib/api.js';
import { materializeOptions } from '$lib/recommendation-materialize.js';
import type { ActionItem } from './action-items.js';

export type ActionItemsApi = Pick<
    SenseiApi,
    'tryPreviewRecommendation' | 'tryMaterializeRecommendationWithReason' | 'trySnoozeWorkspaceRecommendation'
>;

export interface ActionItemsDeps {
    api: ActionItemsApi;
    workspaceId: () => string;
    /** Show a file in Finder (the Tauri opener). */
    reveal: (path: string) => Promise<void>;
    /** Drop a decided recommendation from the list it came from. */
    settle: (recId: string) => void;
}

/** What the person is asked to confirm: what will be written, and where. */
export interface ConfirmView {
    kind: 'rule' | 'skill' | 'agent';
    /** The rule's title, or the file's name. */
    heading: string;
    /** The rule's text, or the file's content. */
    body: string;
    /** The file's path, relative to the repository; null for a rule. */
    path: string | null;
    /** Where a rule lands, e.g. "workspace · recommended"; null for a file. */
    scope: string | null;
    confirmLabel: string;
}

export type ItemFlow =
    | { status: 'idle' }
    | { status: 'working' }
    | { status: 'confirm'; confirm: ConfirmView }
    | { status: 'error'; message: string };

const IDLE: ItemFlow = Object.freeze({ status: 'idle' });
const WORKING: ItemFlow = Object.freeze({ status: 'working' });

/** What the panel renders from and calls: the state's surface, nothing else. */
export type ItemActions = Pick<ActionItemsState, 'done' | 'doneIn' | 'flow' | 'act' | 'confirm' | 'cancel'>;

/** The confirm view a preview implies. A consent-required preview is a file,
 *  whatever the item guessed; anything else is a rule. */
function confirmView(preview: RecommendationPreview, item: ActionItem): ConfirmView {
    if (preview.consent_required) {
        return {
            kind: preview.kind === 'agent' ? 'agent' : 'skill',
            heading: preview.name ?? item.title,
            body: preview.content ?? '',
            path: preview.path ?? null,
            scope: null,
            confirmLabel: 'Write the file',
        };
    }
    const scope = [preview.gov_scope, preview.enforcement].filter(Boolean).join(' · ');
    return {
        kind: 'rule',
        heading: preview.title ?? item.title,
        body: preview.body ?? '',
        path: null,
        scope: scope || null,
        confirmLabel: 'Add the rule',
    };
}

/** A wake time as the person's own calendar date (YYYY-MM-DD). */
const wakeDate = (iso: string): string => new Date(iso).toLocaleDateString('en-CA');

export class ActionItemsState {
    /** What the last settled action did, for the panel to say. */
    done = $state<string | null>(null);

    #flows = new SvelteMap<string, ItemFlow>();
    /** The preview each item is being asked to confirm, as the daemon sent it. */
    #previews = new Map<string, RecommendationPreview>();

    constructor(private readonly deps: ActionItemsDeps) {}

    /** pending: actionitems-lows — what the last settled action did, for the
     *  panel it was acted in (`scope`, the metric's detail) only. */
    doneIn(_scope: string): string | null {
        throw new Error('pending: actionitems-lows');
    }

    flow(id: string): ItemFlow {
        return this.#flows.get(id) ?? IDLE;
    }

    /** Do the item's action. A second call while one is in flight is ignored;
     *  after a failure (or a cancel) it starts again from the top. */
    async act(item: ActionItem, _scope?: string): Promise<void> {
        if (this.flow(item.id).status === 'working') return;
        const { api, workspaceId } = this.deps;
        const action = item.action;
        switch (action.kind) {
            case 'open-lens':
                return; // a link: the panel navigates, nothing to do here
            case 'open-file': {
                this.#flows.set(item.id, WORKING);
                try {
                    await this.deps.reveal(action.path);
                    this.#flows.delete(item.id);
                } catch (e) {
                    const why = e instanceof Error ? e.message : String(e);
                    this.#fail(item.id, `Couldn’t open the file — ${why}`);
                }
                return;
            }
            case 'snooze': {
                this.#flows.set(item.id, WORKING);
                const r = await api.trySnoozeWorkspaceRecommendation(workspaceId(), item.id, action.days);
                if (!r.ok) return this.#fail(item.id, failureReason(r.error));
                this.#settled(item.id, `Snoozed until ${wakeDate(r.data.snoozed_until)}`);
                return;
            }
            case 'propose-rule':
            case 'review-file': {
                this.#flows.set(item.id, WORKING);
                this.#previews.delete(item.id);
                const r = await api.tryPreviewRecommendation(workspaceId(), item.id);
                if (!r.ok) return this.#fail(item.id, failureReason(r.error));
                if (!r.data.materializable) {
                    return this.#fail(item.id, r.data.reason ?? 'sensei can’t apply this recommendation');
                }
                this.#previews.set(item.id, r.data);
                this.#flows.set(item.id, { status: 'confirm', confirm: confirmView(r.data, item) });
                return;
            }
        }
    }

    /** Write what the person was shown. Only from a confirm. */
    async confirm(item: ActionItem, _scope?: string): Promise<void> {
        const flow = this.flow(item.id);
        const preview = this.#previews.get(item.id);
        if (flow.status !== 'confirm' || !preview) return;
        this.#flows.set(item.id, WORKING);
        const r = await this.deps.api.tryMaterializeRecommendationWithReason(
            this.deps.workspaceId(),
            item.id,
            materializeOptions(preview),
        );
        if (!r.ok) return this.#fail(item.id, failureReason(r.error));
        const { confirm } = flow;
        this.#settled(item.id, confirm.path ? `Wrote ${confirm.path}` : `Rule added: ${confirm.heading}`);
    }

    /** Back out of a confirm: nothing is written; the item stays pending. */
    cancel(item: ActionItem): void {
        this.#flows.delete(item.id);
        this.#previews.delete(item.id);
    }

    #fail(id: string, message: string): void {
        this.#previews.delete(id);
        this.#flows.set(id, { status: 'error', message });
    }

    #settled(id: string, message: string): void {
        this.#flows.delete(id);
        this.#previews.delete(id);
        this.deps.settle(id);
        this.done = message;
    }
}
