// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import ActionItemsHarness from './ActionItems.harness.svelte';
import type { ActionItem, ActionUrgency } from './action-items.js';

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
        ...over,
    };
}

function mount(items: ActionItem[], error: string | null = null) {
    const m = mountComponent(ActionItemsHarness, { items, error });
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
        expect(chip('high')?.className).toContain('text-danger');
        expect(chip('medium')?.className).toContain('text-warning');
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
});
