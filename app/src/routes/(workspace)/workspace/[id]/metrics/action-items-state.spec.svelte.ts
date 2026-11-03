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
        tryMaterializeRecommendation: vi.fn(() => Promise.resolve(ok({ ok: true }))),
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
    it.skip('pending: actionitems — proposing a rule previews it and writes nothing yet', async () => {
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
        expect(api.tryMaterializeRecommendation).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it.skip('pending: actionitems — the item shows it is working while the preview is read', async () => {
        const held = deferred<unknown>();
        const { state } = setup({ tryPreviewRecommendation: vi.fn(() => held.promise) });
        const acting = state.act(RULE);
        flushSync();
        expect(state.flow('r1')).toEqual({ status: 'working' });
        held.resolve(ok(RULE_PREVIEW));
        await acting;
        expect(state.flow('r1').status).toBe('confirm');
    });

    it.skip('pending: actionitems — confirming a rule materializes it at workspace scope, recommended, and settles the item', async () => {
        const { api, settle, state } = setup();
        await state.act(RULE);
        await state.confirm(RULE);
        expect(api.tryMaterializeRecommendation).toHaveBeenCalledWith('w1', 'r1', {
            gov_scope: 'workspace',
            enforcement: 'recommended',
        });
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Rule added: Guard the auth branches');
        expect(state.flow('r1')).toEqual({ status: 'idle' });
    });

    it.skip('pending: actionitems — a skill is shown with its path and content, and written only on confirm', async () => {
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
        expect(api.tryMaterializeRecommendation).not.toHaveBeenCalled();
        await state.confirm(SKILL);
        expect(api.tryMaterializeRecommendation).toHaveBeenCalledWith('w1', 'r1', {});
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Wrote .claude/skills/retry-budget/SKILL.md');
    });

    it.skip('pending: actionitems — cancel leaves the recommendation pending and writes nothing', async () => {
        const { api, settle, state } = setup();
        await state.act(RULE);
        state.cancel(RULE);
        expect(state.flow('r1')).toEqual({ status: 'idle' });
        expect(api.tryMaterializeRecommendation).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it.skip('pending: actionitems — a preview that cannot be read is the item’s error, never a write', async () => {
        const { api, settle, state } = setup({
            tryPreviewRecommendation: vi.fn(() => Promise.resolve(fail(500, 'Internal Server Error'))),
        });
        await state.act(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'Internal Server Error' });
        expect(api.tryMaterializeRecommendation).not.toHaveBeenCalled();
        expect(settle).not.toHaveBeenCalled();
    });

    it.skip('pending: actionitems — a recommendation the daemon cannot materialize says why', async () => {
        const { api, state } = setup({
            tryPreviewRecommendation: vi.fn(() =>
                Promise.resolve(ok({ materializable: false, action_type: 'cross_workspace', reason: 'not a materializable action' }))),
        });
        await state.act(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'not a materializable action' });
        expect(api.tryMaterializeRecommendation).not.toHaveBeenCalled();
    });

    it.skip('pending: actionitems — a failed materialize is the item’s error and the item stays', async () => {
        const { settle, state } = setup({
            tryMaterializeRecommendation: vi.fn(() => Promise.resolve(fail(409, 'recommendation not found or already decided'))),
        });
        await state.act(RULE);
        await state.confirm(RULE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'recommendation not found or already decided' });
        expect(settle).not.toHaveBeenCalled();
        expect(state.done).toBeNull();
    });

    it.skip('pending: actionitems — snooze asks the daemon for the item’s days, settles it and says until when', async () => {
        const { api, settle, state } = setup();
        await state.act(SNOOZE);
        expect(api.trySnoozeWorkspaceRecommendation).toHaveBeenCalledWith('w1', 'r1', 7);
        expect(settle).toHaveBeenCalledWith('r1');
        expect(state.done).toBe('Snoozed until 2026-11-10');
    });

    it.skip('pending: actionitems — a failed snooze is the item’s error; the item stays', async () => {
        const { settle, state } = setup({
            trySnoozeWorkspaceRecommendation: vi.fn(() => Promise.resolve(fail(409, 'Conflict'))),
        });
        await state.act(SNOOZE);
        expect(state.flow('r1')).toEqual({ status: 'error', message: 'Conflict' });
        expect(settle).not.toHaveBeenCalled();
    });

    it.skip('pending: actionitems — a second act while the first is in flight writes once', async () => {
        const held = deferred<unknown>();
        const { api, state } = setup({ trySnoozeWorkspaceRecommendation: vi.fn(() => held.promise) });
        const first = state.act(SNOOZE);
        const second = state.act(SNOOZE);
        held.resolve(ok({ ok: true, snoozed_until: '2026-11-10T09:00:00+00:00' }));
        await Promise.all([first, second]);
        expect(api.trySnoozeWorkspaceRecommendation).toHaveBeenCalledOnce();
    });

    it.skip('pending: actionitems — opening a file reveals it and leaves the item in place', async () => {
        const { reveal, settle, state } = setup();
        await state.act(FILE);
        expect(reveal).toHaveBeenCalledWith('/repo/src/lib.rs');
        expect(settle).not.toHaveBeenCalled();
        expect(state.flow('r1')).toEqual({ status: 'idle' });
    });

    it.skip('pending: actionitems — a file that cannot be opened says why, never fails silently', async () => {
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

    it.skip('pending: actionitems — each item keeps its own flow', async () => {
        const { state } = setup({
            trySnoozeWorkspaceRecommendation: vi.fn(() => Promise.resolve(fail(500, 'boom'))),
        });
        await state.act(RULE);
        await state.act(item({ kind: 'snooze', label: 'Snooze 7 days', days: 7 }, 'r2'));
        expect(state.flow('r1').status).toBe('confirm');
        expect(state.flow('r2')).toEqual({ status: 'error', message: 'boom' });
    });
});
