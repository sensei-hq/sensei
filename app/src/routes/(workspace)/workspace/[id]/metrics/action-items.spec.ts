import { describe, it, expect } from 'vitest';
import { buildActionItems, normalizeUrgency } from './action-items.js';
import type { Recommendation } from '$lib/types.js';

function rec(over: Partial<Recommendation> = {}): Recommendation {
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
        const items = buildActionItems([rec({ id: 'a' }), rec({ id: 'b' }), rec({ id: 'c' })]);
        expect(items.map((i) => i.id)).toEqual(['a', 'b', 'c']);
    });

    it('carries title/why/impact and trims them; absent copy stays empty', () => {
        const [i] = buildActionItems([
            rec({ title: '  tighten retries  ', why: '  churny  ', impact: undefined }),
        ]);
        expect(i.title).toBe('tighten retries');
        expect(i.why).toBe('churny');
        expect(i.impact).toBe('');
    });

    it('derives the urgency and its label from the wire urgency', () => {
        expect(buildActionItems([rec({ urgency: 'high' })])[0]).toMatchObject({
            urgency: 'high',
            urgencyLabel: 'high',
        });
        expect(buildActionItems([rec({ urgency: undefined })])[0]).toMatchObject({
            urgency: 'none',
            urgencyLabel: '',
        });
    });

    it('drops rows with no title (nothing to act on), never a blank card', () => {
        const items = buildActionItems([rec({ id: 'keep' }), rec({ id: 'drop', title: '   ' })]);
        expect(items.map((i) => i.id)).toEqual(['keep']);
    });

    it('returns [] for no recommendations (honest-empty)', () => {
        expect(buildActionItems([])).toEqual([]);
    });
});
