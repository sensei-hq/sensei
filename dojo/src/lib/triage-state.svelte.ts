// The reactive selection state for ScrTriage (mockup) — which candidate the
// desktop detail pane shows. Holds the one bit of UI state (`selected`) as
// `$state` and delegates the flatten/lookup to the pure `triage/candidates-view`
// module, so the screen stays presentational. Kept as a `.svelte.ts` rune
// module (like `preview/state.svelte.ts`); the pure helpers unit-test on
// their own in `triage/candidates-view.spec.ts`.

import type { KitTriageCandidate, KitTriageGroup } from './components/kit/types';
import { flattenCandidates } from './triage/candidates-view';

/** The reactive selection state a ScrTriage binds to. */
export interface TriageState {
	/** Every candidate across the scope groups (for the total + default focus). */
	readonly all: KitTriageCandidate[];
	/** The focused candidate id. */
	readonly selected: string;
	/** The focused candidate object (falls back to the first). */
	readonly current: KitTriageCandidate | undefined;
	/** Whether a candidate (by id) is the focused one. */
	isSelected(id: string): boolean;
	/** Focus a candidate. */
	select(id: string): void;
}

/** Build the triage selection state for a dōjō's candidate groups. */
export function createTriage(groups: KitTriageGroup[]): TriageState {
	const all = flattenCandidates(groups);

	let selected = $state(all[0]?.id ?? '');

	const current = $derived(all.find((c) => c.id === selected) ?? all[0]);

	return {
		all,
		get selected() {
			return selected;
		},
		get current() {
			return current;
		},
		isSelected(id: string) {
			return id === selected;
		},
		select(id: string) {
			selected = id;
		}
	};
}
