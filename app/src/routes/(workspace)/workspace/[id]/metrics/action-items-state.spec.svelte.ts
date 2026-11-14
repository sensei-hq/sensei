import { describe, it, expect, vi } from 'vitest';
import { flushSync } from 'svelte';
import { ActionItemsState, type ActionItemsApi } from './action-items-state.svelte.js';
import type { ActionItem, ItemAction } from './action-items.js';

// L3 · ActionItems act (plan 2026-11-03 §L3.1): each item's one action, its
// writes and its failures. Rule/skill/agent go preview → confirm → materialize
// (the path Overview already uses); snooze and open-file are their own.

type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
function deferred<T>(): Deferred<T> {
    let resolve!: (v: T) => void;
    const promise = new Promise<T>((r) => (resolve = r));
    return { promise, resolve };
}
const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });
const tick = async () => {
    for (let i = 0; i < 5; i++) await Promise.resolve();
    flushSync();
};

const RULE_PREVIEW = {
    materializable: true, kind: 'rule', action_type: 'revise_rule',
    title: 'Guard the auth branches', body: 'Every auth branch has a test.',
    gov_scope: 'workspace', enforcement: 'recommended',
};
const SKILL_PREVIEW = {
    materializable: true, kind: 'skill', action_type: 'write_skill',
    name: 'retry-budget', path: '.claude/skills/retry-budget/SKILL.md',
    content: '---\nname: retry-budget\n---\nbody', consent_required: true,
};

function item(action: ItemAction, id = 'r1'): ActionItem {
    return { id, title: 'a recommendation', why: '', impact: '', urgency: 'none', urgencyLabel: '', action };
}
const RULE = item({ kind: 'propose-rule', label: 'Propose a rule' });
const SKILL = item({ kind: 'review-file', label: 'Review the skill' });
const SNOOZE = item({ kind: 'snooze', label: 'Snooze 7 days', days: 7 });
const FILE = item({ kind: 'open-file', label: 'Show lib.rs in Finder', path: '/repo/src/lib.rs' });

function setup(over: Partial<Record<keyof ActionItemsApi, unknown>> = {}) {
    const api = {
        tryPreviewRecommendation: vi.fn(() => Promise.resolve(ok(RULE_PREVIEW))),
        tryMaterializeRecommendationWithReason: vi.fn(() => Promise.resolve(ok({ ok: true }))),
        trySnoozeWorkspaceRecommendation: vi.fn(() =>
            Promise.resolve(ok({ ok: true, snoozed_until: '2026-11-10T09:00:00+00:00' }))),
        ...over,
    } as unknown as ActionItemsApi & Record<string, ReturnType<typeof vi.fn>>;
    const reveal = vi.fn((_path: string) => Promise.resolve());
    const settle = vi.fn((_id: string) => {});
    const state = new ActionItemsState({ api, workspaceId: () => 'w1', reveal, settle });
    return { api, reveal, settle, state };
}

describe('ActionItemsState', () => {
    it('proposing a rule previews it and writes nothing yet', async () => {
        const { api, settle, state } = setup();
        await state.act(RULE);
        expect(api.tryPreviewRecommendation).toHaveBeenCalledWith('w1', 'r1');
        expect(state.flow('r1')).toEqual({
            status: 'confirm',
            confirm: {
                kind: 'rule',
                heading: 'Guard the auth branches',
                body: 'Every auth branch has a test.',
                path: null,
                scope: 'workspace · recommended',
                confirmLabel: 'Add the rule',
            },
        });
        expect(api.tryMaterializeRecommendationWithReason).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it('the item shows it is working while the preview is read', async () => {
        const held = deferred<unknown>();
        const { state } = setup({ tryPreviewRecommendation: vi.fn(() => held.promise) });
        const acting = state.act(RULE);
        flushSync();
        expect(state.flow('r1')).toEqual({ status: 'working' });
        held.resolve(ok(RULE_PREVIEW));
        await acting;
        expect(state.flow('r1').status).toBe('confirm');
    });

    it('confirming a rule materializes it at workspace scope, recommended, and settles the item', async () => {
        const { api, settle, state } = setup();
        await state.act(RULE);
        await state.confirm(RULE);
        expect(api.tryMaterializeRecommendationWithReason).toHaveBeenCalledWith('w1', 'r1', {
            gov_scope: 'workspace',
            enforcement: 'recommended',
        });
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Rule added: Guard the auth branches');
        expect(state.flow('r1')).toEqual({ status: 'idle' });
    });

    it('a skill is shown with its path and content, and written only on confirm', async () => {
        const { api, settle, state } = setup({
            tryPreviewRecommendation: vi.fn(() => Promise.resolve(ok(SKILL_PREVIEW))),
        });
        await state.act(SKILL);
        expect(state.flow('r1')).toEqual({
            status: 'confirm',
            confirm: {
                kind: 'skill',
                heading: 'retry-budget',
                body: SKILL_PREVIEW.content,
                path: '.claude/skills/retry-budget/SKILL.md',
                scope: null,
                confirmLabel: 'Write the file',
            },
        });
        expect(api.tryMaterializeRecommendationWithReason).not.toHaveBeenCalled();
        await state.confirm(SKILL);
        expect(api.tryMaterializeRecommendationWithReason).toHaveBeenCalledWith('w1', 'r1', {});
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Wrote .claude/skills/retry-budget/SKILL.md');
    });

    it('cancel leaves the recommendation pending and writes nothing', async () => {
        const { api, settle, state } = setup();
        await state.act(RULE);
        state.cancel(RULE);
        expect(state.flow('r1')).toEqual({ status: 'idle' });
        expect(api.tryMaterializeRecommendationWithReason).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it('a preview that cannot be read is the item’s error, never a write', async () => {
        const { api, settle, state } = setup({
            tryPreviewRecommendation: vi.fn(() => Promise.resolve(fail(500, 'Internal Server Error'))),
        });
        await state.act(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'Internal Server Error' });
        expect(api.tryMaterializeRecommendationWithReason).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it('a recommendation the daemon cannot materialize says why', async () => {
        const { api, state } = setup({
            tryPreviewRecommendation: vi.fn(() =>
                Promise.resolve(ok({ materializable: false, action_type: 'cross_workspace', reason: 'not a materializable action' }))),
        });
        await state.act(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'not a materializable action' });
        expect(api.tryMaterializeRecommendationWithReason).not.toHaveBeenCalled();
    });

    it('a failed materialize is the item’s error and the item stays', async () => {
        const { settle, state } = setup({
            tryMaterializeRecommendationWithReason: vi.fn(() => Promise.resolve(fail(409, 'recommendation not found or already decided'))),
        });
        await state.act(RULE);
        await state.confirm(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'recommendation not found or already decided' });
        expect(settle).not.toHaveBeenCalled();
        expect(state.done).toBeNull();
    });

    it('snooze asks the daemon for the item’s days, settles it and says until when', async () => {
        const { api, settle, state } = setup();
        await state.act(SNOOZE);
        expect(api.trySnoozeWorkspaceRecommendation).toHaveBeenCalledWith('w1', 'r1', 7);
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Snoozed until 2026-11-10');
    });

    it('a failed snooze is the item’s error; the item stays', async () => {
        const { settle, state } = setup({
            trySnoozeWorkspaceRecommendation: vi.fn(() => Promise.resolve(fail(409, 'Conflict'))),
        });
        await state.act(SNOOZE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'Conflict' });
        expect(settle).not.toHaveBeenCalled();
    });

    it('a second act while the first is in flight writes once', async () => {
        const held = deferred<unknown>();
        const { api, state } = setup({ trySnoozeWorkspaceRecommendation: vi.fn(() => held.promise) });
        const first = state.act(SNOOZE);
        const second = state.act(SNOOZE);
        held.resolve(ok({ ok: true, snoozed_until: '2026-11-10T09:00:00+00:00' }));
        await Promise.all([first, second]);
        expect(api.trySnoozeWorkspaceRecommendation).toHaveBeenCalledOnce();
    });

    it('opening a file reveals it and leaves the item in place', async () => {
        const { reveal, settle, state } = setup();
        await state.act(FILE);
        expect(reveal).toHaveBeenCalledWith('/repo/src/lib.rs');
        expect(settle).not.toHaveBeenCalled();
        expect(state.flow('r1')).toEqual({ status: 'idle' });
    });

    it('a file that cannot be opened says why, never fails silently', async () => {
        const failing = new ActionItemsState({
            api: setup().api,
            workspaceId: () => 'w1',
            reveal: () => Promise.reject(new Error('path not allowed')),
            settle: () => {},
        });
        await failing.act(FILE);
        await tick();
        expect(failing.flow('r1')).toEqual({ status: 'error', message: 'Couldn’t open the file — path not allowed' });
    });

    it('each item keeps its own flow', async () => {
        const { state } = setup({
            trySnoozeWorkspaceRecommendation: vi.fn(() => Promise.resolve(fail(500, 'boom'))),
        });
        await state.act(RULE);
        await state.act(item({ kind: 'snooze', label: 'Snooze 7 days', days: 7 }, 'r2'));
        expect(state.flow('r1').status).toBe('confirm');
        expect(state.flow('r2')).toEqual({ status: 'error', message: 'boom' });
    });

    // ── Review r1 M4: the preview decides, never the item's guessed kind ──────

    it('a propose-rule item whose preview is file-class is confirmed and sent as a file', async () => {
        const { api, state } = setup({
            tryPreviewRecommendation: vi.fn(() => Promise.resolve(ok(SKILL_PREVIEW))),
        });
        await state.act(RULE);
        const flow = state.flow('r1');
        expect(flow.status === 'confirm' && flow.confirm).toMatchObject({
            kind: 'skill',
            path: '.claude/skills/retry-budget/SKILL.md',
            confirmLabel: 'Write the file',
        });
        await state.confirm(RULE);
        expect(api.tryMaterializeRecommendationWithReason).toHaveBeenCalledWith('w1', 'r1', {});
    });

    it('a rule is added at the scope and tier its preview proposed', async () => {
        const { api, state } = setup({
            tryPreviewRecommendation: vi.fn(() =>
                Promise.resolve(ok({ ...RULE_PREVIEW, gov_scope: 'repository', enforcement: 'advisory' }))),
        });
        await state.act(RULE);
        const flow = state.flow('r1');
        expect(flow.status === 'confirm' && flow.confirm.scope).toBe('repository · advisory');
        await state.confirm(RULE);
        expect(api.tryMaterializeRecommendationWithReason).toHaveBeenCalledWith('w1', 'r1', {
            gov_scope: 'repository',
            enforcement: 'advisory',
        });
    });

    // ── Review r1 M1: a failed item can be retried, and the retry writes ──────

    it('after a failed snooze, acting again retries and settles', async () => {
        const snooze = vi.fn()
            .mockResolvedValueOnce(fail(500, 'boom'))
            .mockResolvedValueOnce(ok({ ok: true, snoozed_until: '2026-11-10T09:00:00+00:00' }));
        const { settle, state } = setup({ trySnoozeWorkspaceRecommendation: snooze });
        await state.act(SNOOZE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'boom' });
        await state.act(SNOOZE);
        expect(snooze).toHaveBeenCalledTimes(2);
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Snoozed until 2026-11-10');
        expect(state.flow('r1').status).not.toBe('error');
    });

    it('after a failed materialize, acting again previews afresh and the confirm writes', async () => {
        const materialize = vi.fn()
            .mockResolvedValueOnce(fail(409, "a skill named 'retry-budget' already exists in this repo; edit the title"))
            .mockResolvedValueOnce(ok({ ok: true }));
        const { api, settle, state } = setup({
            tryPreviewRecommendation: vi.fn(() => Promise.resolve(ok(SKILL_PREVIEW))),
            tryMaterializeRecommendationWithReason: materialize,
        });
        await state.act(SKILL);
        await state.confirm(SKILL);
        expect(state.flow('r1')).toEqual({
            status: 'error',
            message: "a skill named 'retry-budget' already exists in this repo; edit the title",
        });
        await state.act(SKILL);
        expect(api.tryPreviewRecommendation).toHaveBeenCalledTimes(2);
        expect(state.flow('r1').status).toBe('confirm');
        await state.confirm(SKILL);
        expect(materialize).toHaveBeenCalledTimes(2);
        expect(settle).toHaveBeenCalledWith('r1');
    });

    // ActionItems review r1 LOW-2: the outcome line belongs to the panel it was
    // acted in, so opening another metric never shows "Snoozed until …" for an
    // item that was never in its list.
    it('what was done is said only in the scope it was done in', async () => {
        const { state } = setup();
        await state.act(SNOOZE, 'ftr');
        expect(state.doneIn('ftr')).toBe('Snoozed until 2026-11-10');
        expect(state.doneIn('churn_rate')).toBeNull();

        await state.act(RULE, 'churn_rate');
        await state.confirm(RULE, 'churn_rate');
        expect(state.doneIn('churn_rate')).toBe('Rule added: Guard the auth branches');
        expect(state.doneIn('ftr')).toBeNull();
    });
});
