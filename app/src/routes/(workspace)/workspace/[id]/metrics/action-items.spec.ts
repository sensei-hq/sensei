import { describe, it, expect } from 'vitest';
import { actionFor, buildActionItems, normalizeUrgency, SNOOZE_DAYS } from './action-items.js';
import type { ActionableRecommendation } from './action-items.js';

const W = 'w1';

function rec(over: Partial<ActionableRecommendation> = {}): ActionableRecommendation {
    return { id: 'r1', title: 'tighten the retry budget', status: 'pending', ...over };
}

describe('normalizeUrgency', () => {
    it('maps the wire levels, case- and space-insensitively', () => {
        expect(normalizeUrgency('high')).toBe('high');
        expect(normalizeUrgency(' Medium ')).toBe('medium');
        expect(normalizeUrgency('LOW')).toBe('low');
    });

    it('maps an absent or unrecognized level to none (no fabricated urgency)', () => {
        expect(normalizeUrgency(undefined)).toBe('none');
        expect(normalizeUrgency(null)).toBe('none');
        expect(normalizeUrgency('')).toBe('none');
        expect(normalizeUrgency('urgent')).toBe('none');
    });
});

describe('buildActionItems', () => {
    it('maps recommendations to action items, preserving wire order', () => {
        const items = buildActionItems([rec({ id: 'a' }), rec({ id: 'b' }), rec({ id: 'c' })], W);
        expect(items.map((i) => i.id)).toEqual(['a', 'b', 'c']);
    });

    it('carries title/why/impact and trims them; absent copy stays empty', () => {
        const [i] = buildActionItems([
            rec({ title: '  tighten retries  ', why: '  churny  ', impact: undefined }),
        ], W);
        expect(i.title).toBe('tighten retries');
        expect(i.why).toBe('churny');
        expect(i.impact).toBe('');
    });

    it('derives the urgency and its label from the wire urgency', () => {
        expect(buildActionItems([rec({ urgency: 'high' })], W)[0]).toMatchObject({
            urgency: 'high',
            urgencyLabel: 'high',
        });
        expect(buildActionItems([rec({ urgency: undefined })], W)[0]).toMatchObject({
            urgency: 'none',
            urgencyLabel: '',
        });
    });

    it('drops rows with no title (nothing to act on), never a blank card', () => {
        const items = buildActionItems([rec({ id: 'keep' }), rec({ id: 'drop', title: '   ' })], W);
        expect(items.map((i) => i.id)).toEqual(['keep']);
    });

    it('returns [] for no recommendations (honest-empty)', () => {
        expect(buildActionItems([], W)).toEqual([]);
    });
});

// L3 · ActionItems act: each item gets the one action its recommendation implies
// (plan 2026-11-03 §L3.1). The action is chosen from the wire `actionType`; the
// rule/skill/agent ones reuse the existing preview → materialize path.
describe('actionFor', () => {
    const kindOf = (actionType: string | null, file: string | null = null) =>
        actionFor(rec({ actionType, file }), W).kind;

    it.skip('pending: actionitems — a rule-class recommendation proposes a rule', () => {
        for (const t of ['revise_rule', 'promote_pattern', 'enrich_memory']) {
            expect(actionFor(rec({ actionType: t }), W)).toEqual({ kind: 'propose-rule', label: 'Propose a rule' });
        }
    });

    it.skip('pending: actionitems — a skill or agent recommendation is reviewed before its file is written', () => {
        expect(actionFor(rec({ actionType: 'write_skill' }), W)).toEqual({ kind: 'review-file', label: 'Review the skill' });
        expect(actionFor(rec({ actionType: 'create_agent' }), W)).toEqual({ kind: 'review-file', label: 'Review the agent' });
    });

    it.skip('pending: actionitems — a rework hotspot opens its file, named by its base name', () => {
        expect(actionFor(rec({ actionType: 'audit_stale', file: '/repo/crates/x/src/lib.rs' }), W)).toEqual({
            kind: 'open-file',
            label: 'Show lib.rs in Finder',
            path: '/repo/crates/x/src/lib.rs',
        });
    });

    it.skip('pending: actionitems — a library update opens the workspace’s Libraries lens', () => {
        expect(actionFor(rec({ actionType: 'library_update' }), W)).toEqual({
            kind: 'open-lens',
            label: 'Open libraries',
            href: '/workspace/w1/libraries',
        });
        expect(actionFor(rec({ actionType: 'library_update' }), 'a b')).toMatchObject({
            href: '/workspace/a%20b/libraries',
        });
    });

    it.skip('pending: actionitems — a rework hotspot whose file the daemon could not resolve is snoozed, never opened at a guessed path', () => {
        // The title names the file ("High rework: …"), but parsing copy is not a path.
        const a = actionFor(rec({ actionType: 'audit_stale', file: null, title: 'High rework: src/a.rs' }), W);
        expect(a).toEqual({ kind: 'snooze', label: `Snooze ${SNOOZE_DAYS} days`, days: SNOOZE_DAYS });
        expect(kindOf('audit_stale', '   ')).toBe('snooze');
    });

    it.skip('pending: actionitems — anything without a sharper action is snoozed (cross_workspace, unknown, absent)', () => {
        expect(kindOf('cross_workspace')).toBe('snooze');
        expect(kindOf('something_new')).toBe('snooze');
        expect(kindOf(null)).toBe('snooze');
    });

    it.skip('pending: actionitems — buildActionItems carries each item’s action', () => {
        const items = buildActionItems(
            [rec({ id: 'a', actionType: 'revise_rule' }), rec({ id: 'b', actionType: 'cross_workspace' })],
            W,
        );
        expect(items.map((i) => i.action.kind)).toEqual(['propose-rule', 'snooze']);
    });
});
