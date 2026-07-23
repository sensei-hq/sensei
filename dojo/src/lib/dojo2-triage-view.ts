// Pure presentation helpers for the maintainer triage console (ScrTriage,
// mockup). Side-effect-free (data in → display value out) so the grouping, the
// impact tone, and the second-approval rule unit-test without a DOM, and the
// selection rune store (`dojo2-triage-state.svelte.ts`) stays a thin wrapper.
//
// Colour is a named-token utility CLASS (`text-accent` · `bg-danger-soft` · …)
// per the design system — never a raw oklch. This is the token-native port of
// the mockup's `K2_IMPACT` map.

import type { KitTriageCandidate, KitTriageGroup } from './components/kit/types';

/** Every candidate across the scope groups, in group order (mockup
 *  `groups.flatMap(g => g.items)`). Backs the default selection + the total. */
export function flattenCandidates(groups: KitTriageGroup[]): KitTriageCandidate[] {
	return groups.flatMap((g) => g.items);
}

/** The token tone for an impact chip / row marker. */
export interface ImpactTone {
	/** Foreground text token class. */
	text: string;
	/** Background fill token class (soft tint). */
	soft: string;
	/** Border token class (edge tint). */
	edge: string;
}

const IMPACT_TONES: Record<string, ImpactTone> = {
	high: { text: 'text-accent', soft: 'bg-accent-soft', edge: 'border-accent-soft' },
	safety: { text: 'text-danger', soft: 'bg-danger-soft', edge: 'border-danger-soft' }
};

const NEUTRAL_IMPACT: ImpactTone = {
	text: 'text-ink-mute',
	soft: 'bg-paper-mute',
	edge: 'border-paper-edge'
};

/** Token classes for an impact level (mockup `K2_IMPACT`). high → accent,
 *  safety → danger, everything else (normal · low · unknown) → neutral ink. */
export function impactTone(impact: string): ImpactTone {
	return IMPACT_TONES[impact] ?? NEUTRAL_IMPACT;
}

/** Whether a candidate of this impact routes to a second maintainer's signature
 *  before it publishes (mockup: high + safety). */
export function needsSecondApproval(impact: string): boolean {
	return impact === 'high' || impact === 'safety';
}
