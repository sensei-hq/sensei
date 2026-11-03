// Metric detail · action items — the state behind each item's one action. The
// component renders `flow(id)` and calls `act` / `confirm` / `cancel`; every
// write goes through here.
//
// - propose-rule / review-file: preview first (nothing is written), then the
//   person confirms and the recommendation is materialized.
// - snooze: the daemon hides the recommendation until it wakes.
// - open-file: the file is shown in Finder.
// A settled item leaves the list (`settle`) and `done` says what happened. A
// failure is that item's error, never a silent no-op.
import type { SenseiApi } from '$lib/api.js';
import type { ActionItem } from './action-items.js';

export type ActionItemsApi = Pick<
    SenseiApi,
    'tryPreviewRecommendation' | 'tryMaterializeRecommendation' | 'trySnoozeWorkspaceRecommendation'
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

/** What the panel renders from and calls: the state's surface, nothing else. */
export type ItemActions = Pick<ActionItemsState, 'done' | 'flow' | 'act' | 'confirm' | 'cancel'>;

export class ActionItemsState {
    /** What the last settled action did, for the panel to say. */
    done = $state<string | null>(null);

    constructor(private readonly deps: ActionItemsDeps) {}

    /** pending: actionitems */
    flow(_id: string): ItemFlow {
        return IDLE;
    }

    /** pending: actionitems */
    async act(_item: ActionItem): Promise<void> {
        throw new Error('pending: actionitems');
    }

    /** pending: actionitems */
    async confirm(_item: ActionItem): Promise<void> {
        throw new Error('pending: actionitems');
    }

    /** pending: actionitems */
    cancel(_item: ActionItem): void {
        throw new Error('pending: actionitems');
    }
}
