// Metric detail · action items — pure view logic behind `ActionItems.svelte`,
// so the component stays a dumb template (props in, markup out — mirrors
// `metric-view.ts`'s card/signal mappers).
//
// The detail screen pairs each observation with an action item: the workspace's
// PENDING recommendations, already score-ranked by the daemon. This module maps
// the wire `Recommendation` rows into presentational view-models.
//
// Governance (no fabrication): every field is the recommendation's own copy or
// honest-absent. An absent `why`/`impact` stays '' (the row omits that line); an
// absent or unrecognized `urgency` maps to 'none' (no chip), never an invented
// level. A row with no title carries nothing to act on and is dropped rather
// than rendered as a blank card. A failed read is the panel's error state (the
// metrics state's section), so this never sees a fabricated row.
//
// Each item also carries its one action (`actionFor`); the writes behind it are
// `action-items-state.svelte.ts`'s.

import type { Recommendation } from '$lib/types.js';
import { SECTIONS } from '../workspace-sidebar-view.js';

/** Urgency level for an action item's chip. `none` → no chip (the wire sent an
 *  absent/unrecognized level — never a fabricated one). */
export type ActionUrgency = 'high' | 'medium' | 'low' | 'none';

/** How long a snooze hides an item. */
export const SNOOZE_DAYS = 7;

/** The one thing an item lets you do, chosen from what the recommendation implies. */
export type ItemAction =
    | { kind: 'propose-rule'; label: string }
    | { kind: 'review-file'; label: string }
    | { kind: 'open-file'; label: string; path: string }
    | { kind: 'open-lens'; label: string; href: string }
    | { kind: 'snooze'; label: string; days: number };

/** The analyzer's rule-class actions: accepting one writes a governance rule. */
const RULE_ACTIONS: ReadonlySet<string> = new Set(['revise_rule', 'promote_pattern', 'enrich_memory']);

/** The analyzer's file-class actions, and the artifact each writes. */
const FILE_ACTIONS: Readonly<Record<string, string>> = { write_skill: 'skill', create_agent: 'agent' };

/** The workspace window's Libraries lens, from the sections it owns. */
const LIBRARIES = SECTIONS.find((s) => s.id === 'libraries');

const snooze = (): ItemAction => ({ kind: 'snooze', label: `Snooze ${SNOOZE_DAYS} days`, days: SNOOZE_DAYS });

/**
 * The one action a recommendation implies. Rule- and file-class ones are
 * previewed before anything is written (the preview then decides what is
 * confirmed); a rework hotspot opens its file, but only at the path the daemon
 * resolved (never one parsed from the title); a library update opens the
 * Libraries lens. Anything else is snoozed.
 */
export function actionFor(rec: Recommendation, workspaceId: string): ItemAction {
    const type = rec.actionType ?? '';
    if (RULE_ACTIONS.has(type)) return { kind: 'propose-rule', label: 'Propose a rule' };
    if (FILE_ACTIONS[type]) return { kind: 'review-file', label: `Review the ${FILE_ACTIONS[type]}` };
    if (type === 'audit_stale') {
        const path = rec.file?.trim();
        if (!path) return snooze();
        const name = path.split('/').pop() || path;
        return { kind: 'open-file', label: `Show ${name} in Finder`, path };
    }
    if (type === 'library_update' && LIBRARIES) {
        return {
            kind: 'open-lens',
            label: `Open ${LIBRARIES.label.toLowerCase()}`,
            href: `/workspace/${encodeURIComponent(workspaceId)}/${LIBRARIES.id}`,
        };
    }
    return snooze();
}

/** A presentational action item — the metric-detail card's render shape. */
export interface ActionItem {
    /** What the item lets you do. */
    action: ItemAction;
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
 * Map the workspace's pending recommendations to action items, preserving the
 * wire order (the daemon already score-ranks them — the client never re-ranks).
 * Rows with no title are dropped (nothing to act on), never a blank card.
 */
export function buildActionItems(recs: Recommendation[], workspaceId: string): ActionItem[] {
    return recs
        .filter((r) => r.title?.trim())
        .map((r) => {
            const urgency = normalizeUrgency(r.urgency);
            return {
                action: actionFor(r, workspaceId),
                id: r.id,
                title: r.title.trim(),
                why: r.why?.trim() ?? '',
                impact: r.impact?.trim() ?? '',
                urgency,
                urgencyLabel: urgency === 'none' ? '' : URGENCY_LABEL[urgency],
            };
        });
}
