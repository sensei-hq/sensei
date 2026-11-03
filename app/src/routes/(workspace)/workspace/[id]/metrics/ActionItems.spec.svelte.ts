// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import ActionItemsHarness from './ActionItems.harness.svelte';
import type { ActionItem, ActionUrgency, ItemAction } from './action-items.js';
import type { ConfirmView, ItemActions, ItemFlow } from './action-items-state.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

function item(over: Partial<ActionItem> = {}): ActionItem {
    return {
        id: 'r1',
        title: 'add regression tests around the auth guard',
        why: 'three of the last five corrections touched untested guard branches',
        impact: 'projected +8 pt ftr on auth-touching sessions',
        urgency: 'high',
        urgencyLabel: 'high',
        action: { kind: 'snooze', label: 'Snooze 7 days', days: 7 },
        ...over,
    };
}

/** A stand-in for ActionItemsState: flows set per item, calls recorded. */
function fakeActions(flows: Record<string, ItemFlow> = {}, done: string | null = null) {
    return {
        done,
        flow: vi.fn((id: string): ItemFlow => flows[id] ?? { status: 'idle' }),
        act: vi.fn(async () => {}),
        confirm: vi.fn(async () => {}),
        cancel: vi.fn(() => {}),
    } satisfies ItemActions;
}

function mount(items: ActionItem[], error: string | null = null, actions: ItemActions = fakeActions()) {
    const m = mountComponent(ActionItemsHarness, { items, error, actions });
    cleanup.push(m.destroy);
    return m.container;
}

const q = (root: HTMLElement, sel: string) => root.querySelector(sel) as HTMLElement | null;
const qa = (root: HTMLElement, sel: string) => Array.from(root.querySelectorAll(sel)) as HTMLElement[];

describe('ActionItems', () => {
    it('renders each item with its title, why and impact', () => {
        const root = mount([item()]);
        expect(q(root, '[data-action-item="r1"]')?.textContent).toContain('auth guard');
        expect(q(root, '[data-action-why]')?.textContent).toContain('untested guard');
        expect(q(root, '[data-action-impact]')?.textContent).toContain('+8 pt');
    });

    it('maps urgency to a named-token chip (high→danger, medium→warning, low→ink-faint)', () => {
        const chip = (u: ActionUrgency) =>
            q(mount([item({ urgency: u, urgencyLabel: u })]), '[data-action-urgency]');
        expect(chip('high')?.className).toContain('text-on-danger-soft');
        expect(chip('medium')?.className).toContain('text-on-warning-soft');
        expect(chip('low')?.className).toContain('text-ink-faint');
    });

    it('carries the urgency level on a data attribute and shows its label', () => {
        const chip = q(
            mount([item({ urgency: 'medium', urgencyLabel: 'medium' })]),
            '[data-action-urgency]',
        );
        expect(chip?.getAttribute('data-action-urgency')).toBe('medium');
        expect(chip?.textContent?.trim()).toBe('medium');
    });

    it('omits the chip when urgency is none (never a fabricated level)', () => {
        expect(
            q(mount([item({ urgency: 'none', urgencyLabel: '' })]), '[data-action-urgency]'),
        ).toBeNull();
    });

    it('omits the why / impact lines when the wire sent none (no blank rows)', () => {
        const root = mount([item({ why: '', impact: '' })]);
        expect(q(root, '[data-action-why]')).toBeNull();
        expect(q(root, '[data-action-impact]')).toBeNull();
    });

    it('renders items in the given (score-ranked) order', () => {
        const root = mount([
            item({ id: 'a', title: 'first' }),
            item({ id: 'b', title: 'second' }),
            item({ id: 'c', title: 'third' }),
        ]);
        const ids = qa(root, '[data-action-item]').map((el) => el.getAttribute('data-action-item'));
        expect(ids).toEqual(['a', 'b', 'c']);
    });

    it('shows the honest empty line when there are no action items', () => {
        const root = mount([]);
        expect(q(root, '[data-action-items-empty]')?.textContent).toContain('No action items yet');
        expect(q(root, '[data-action-item]')).toBeNull();
    });

    it('shows an ERROR state on a failed recs fetch — distinct from the empty state (no masked failure)', () => {
        const root = mount([], 'Internal Server Error');
        expect(q(root, '[data-action-items-error]')?.textContent).toContain('Internal Server Error');
        // Never the "no action items" line for a failure — that would mask it.
        expect(q(root, '[data-action-items-empty]')).toBeNull();
        expect(q(root, '[data-action-item]')).toBeNull();
    });

    // ── L3 · ActionItems act (plan 2026-11-03 §L3.1) ─────────────────────

    const withAction = (action: ItemAction, id = 'r1') => item({ id, action });
    const RULE_CONFIRM: ConfirmView = {
        kind: 'rule', heading: 'Guard the auth branches', body: 'Every auth branch has a test.',
        path: null, scope: 'workspace · recommended', confirmLabel: 'Add the rule',
    };
    const FILE_CONFIRM: ConfirmView = {
        kind: 'skill', heading: 'retry-budget', body: '---\nname: retry-budget\n---',
        path: '.claude/skills/retry-budget/SKILL.md', scope: null, confirmLabel: 'Write the file',
    };

    it.skip('pending: actionitems — every item has its one action, labelled by what it does', () => {
        const root = mount([
            withAction({ kind: 'propose-rule', label: 'Propose a rule' }, 'a'),
            withAction({ kind: 'review-file', label: 'Review the skill' }, 'b'),
            withAction({ kind: 'open-file', label: 'Show lib.rs in Finder', path: '/r/lib.rs' }, 'c'),
            withAction({ kind: 'snooze', label: 'Snooze 7 days', days: 7 }, 'd'),
        ]);
        const label = (id: string) =>
            q(root, `[data-action-item="${id}"] button[data-action-do]`)?.textContent?.trim();
        expect(label('a')).toBe('Propose a rule');
        expect(label('b')).toBe('Review the skill');
        expect(label('c')).toBe('Show lib.rs in Finder');
        expect(label('d')).toBe('Snooze 7 days');
        expect(q(root, '[data-action-item="a"] [data-action-do]')?.getAttribute('data-action-do')).toBe('propose-rule');
    });

    it.skip('pending: actionitems — a lens action is a link to the lens, not a button', () => {
        const root = mount([withAction({ kind: 'open-lens', label: 'Open libraries', href: '/workspace/w1/libraries' })]);
        const link = q(root, '[data-action-item="r1"] a[data-action-do="open-lens"]');
        expect(link?.getAttribute('href')).toBe('/workspace/w1/libraries');
        expect(link?.textContent?.trim()).toBe('Open libraries');
        expect(q(root, '[data-action-item="r1"] button[data-action-do]')).toBeNull();
    });

    it.skip('pending: actionitems — clicking the action hands the item to the state', () => {
        const actions = fakeActions();
        const it1 = withAction({ kind: 'snooze', label: 'Snooze 7 days', days: 7 });
        const root = mount([it1], null, actions);
        q(root, 'button[data-action-do="snooze"]')!.click();
        flushSync();
        expect(actions.act).toHaveBeenCalledWith(it1);
    });

    it.skip('pending: actionitems — a working item disables its action and says it is busy', () => {
        const root = mount([item()], null, fakeActions({ r1: { status: 'working' } }));
        const btn = q(root, 'button[data-action-do]') as HTMLButtonElement | null;
        expect(btn?.disabled).toBe(true);
        expect(btn?.getAttribute('aria-busy')).toBe('true');
    });

    it.skip('pending: actionitems — a rule to confirm shows its title, text and where it lands, with Add and Cancel', () => {
        const actions = fakeActions({ r1: { status: 'confirm', confirm: RULE_CONFIRM } });
        const it1 = withAction({ kind: 'propose-rule', label: 'Propose a rule' });
        const root = mount([it1], null, actions);
        const panel = q(root, '[data-action-confirm="rule"]');
        expect(panel?.textContent).toContain('Guard the auth branches');
        expect(panel?.textContent).toContain('Every auth branch has a test.');
        expect(q(root, '[data-action-confirm-scope]')?.textContent).toContain('workspace · recommended');
        expect(q(root, '[data-action-confirm-path]')).toBeNull();
        q(root, 'button[data-action-confirm-yes]')!.click();
        flushSync();
        expect(actions.confirm).toHaveBeenCalledWith(it1);
        expect(q(root, 'button[data-action-confirm-yes]')?.textContent?.trim()).toBe('Add the rule');
        q(root, 'button[data-action-confirm-no]')!.click();
        flushSync();
        expect(actions.cancel).toHaveBeenCalledWith(it1);
    });

    it.skip('pending: actionitems — a file to confirm shows the path it will write and its full content', () => {
        const root = mount(
            [withAction({ kind: 'review-file', label: 'Review the skill' })],
            null,
            fakeActions({ r1: { status: 'confirm', confirm: FILE_CONFIRM } }),
        );
        expect(q(root, '[data-action-confirm-path]')?.textContent).toContain('.claude/skills/retry-budget/SKILL.md');
        expect(q(root, '[data-action-confirm="skill"] pre')?.textContent).toContain('name: retry-budget');
        expect(q(root, 'button[data-action-confirm-yes]')?.textContent?.trim()).toBe('Write the file');
    });

    it.skip('pending: actionitems — an item’s failure is shown on that item, and its action stays to retry', () => {
        const root = mount(
            [item({ id: 'a' }), item({ id: 'b' })],
            null,
            fakeActions({ a: { status: 'error', message: 'Conflict' } }),
        );
        expect(q(root, '[data-action-item="a"] [data-action-error]')?.textContent).toContain('Conflict');
        expect(q(root, '[data-action-item="b"] [data-action-error]')).toBeNull();
        expect(q(root, '[data-action-item="a"] button[data-action-do]')).not.toBeNull();
    });

    it.skip('pending: actionitems — what the last action did is said above the list', () => {
        const root = mount([item()], null, fakeActions({}, 'Snoozed until 2026-11-10'));
        expect(q(root, '[data-action-done]')?.textContent).toContain('Snoozed until 2026-11-10');
        expect(q(mount([item()]), '[data-action-done]')).toBeNull();
        // The last item settled: the list is empty, and the outcome is still said.
        const emptied = mount([], null, fakeActions({}, 'Rule added: Guard the auth branches'));
        expect(q(emptied, '[data-action-done]')?.textContent).toContain('Rule added');
        expect(q(emptied, '[data-action-items-empty]')).not.toBeNull();
    });
});
