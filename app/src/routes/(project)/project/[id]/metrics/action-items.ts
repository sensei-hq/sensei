// Metric detail · action items — pure view logic behind `ActionItems.svelte`,
// so the component stays a dumb template (props in, markup out — mirrors
// `metric-view.ts`'s card/signal mappers).
//
// The detail screen pairs each observation with an action item: the project's
// PENDING recommendations, already score-ranked by the daemon. This module maps
// the wire `Recommendation` rows into presentational view-models.
//
// Governance (no fabrication): every field is the recommendation's own copy or
// honest-absent. An absent `why`/`impact` stays '' (the row omits that line); an
// absent or unrecognized `urgency` maps to 'none' (no chip), never an invented
// level. A row with no title carries nothing to act on and is dropped rather
// than rendered as a blank card. The loader returns honest-[] on a failed fetch,
// so this never sees a fabricated row.

import type { Recommendation } from '$lib/types.js';

/** Urgency level for an action item's chip. `none` → no chip (the wire sent an
 *  absent/unrecognized level — never a fabricated one). */
export type ActionUrgency = 'high' | 'medium' | 'low' | 'none';

/** A presentational action item — the metric-detail card's render shape. */
export interface ActionItem {
    id: string;
    title: string;
    /** Why sensei recommends it; '' when the wire sent none (the row omits it). */
    why: string;
    /** Expected impact; '' when absent (the row omits it). */
    impact: string;
    urgency: ActionUrgency;
    /** Human label for the chip ('' when `urgency` is 'none'). */
    urgencyLabel: string;
}

const URGENCY_LABEL: Record<Exclude<ActionUrgency, 'none'>, string> = {
    high: 'high',
    medium: 'medium',
    low: 'low',
};

/** Normalize the wire's `urgency` to a known level, or 'none' when it is absent
 *  or unrecognized — so the chip is omitted rather than fabricated. */
export function normalizeUrgency(urgency: string | null | undefined): ActionUrgency {
    switch (urgency?.trim().toLowerCase()) {
        case 'high':
            return 'high';
        case 'medium':
            return 'medium';
        case 'low':
            return 'low';
        default:
            return 'none';
    }
}

/**
 * Map the project's pending recommendations to action items, preserving the
 * wire order (the daemon already score-ranks them — the client never re-ranks).
 * Rows with no title are dropped (nothing to act on), never a blank card.
 */
export function buildActionItems(recs: Recommendation[]): ActionItem[] {
    return recs
        .filter((r) => r.title?.trim())
        .map((r) => {
            const urgency = normalizeUrgency(r.urgency);
            return {
                id: r.id,
                title: r.title.trim(),
                why: r.why?.trim() ?? '',
                impact: r.impact?.trim() ?? '',
                urgency,
                urgencyLabel: urgency === 'none' ? '' : URGENCY_LABEL[urgency],
            };
        });
}
