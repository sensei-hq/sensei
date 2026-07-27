// The reactive adopt-toggle state for ScrRulePacks (mockup). Seeds from the
// packs the page supplies and lets the viewer adopt/drop a pack locally
// (presentational this chunk — a later chunk wires it to a `/v1` mutation). The
// adopted / available split is derived off the live `adopted` set, so toggling
// a pack moves it between the two sections. Kept as a `.svelte.ts` rune module
// (like `preview/state.svelte.ts`) so the split is reactive `$state`; the
// pure split lives in `personal-view.ts`.

import type { KitRulePack } from './components/kit/types';
import { splitPacks } from './personal-view';

/** The reactive rule-packs state a ScrRulePacks binds to. */
export interface RulePacksState {
	/** Every pack with its current (possibly toggled) adopted flag. */
	readonly packs: KitRulePack[];
	/** The adopted set, derived off the live flags. */
	readonly adopted: KitRulePack[];
	/** The still-available set, derived off the live flags. */
	readonly available: KitRulePack[];
	/** Whether a pack id is currently adopted. */
	isAdopted(id: string): boolean;
	/** Flip a pack's adopted flag (adopt if available, drop if adopted). */
	toggle(id: string): void;
}

/** Build the adopt-toggle state for a set of rule packs. Seeds ONCE from the
 *  supplied packs (a navigation re-mounts the screen), then owns the flags. */
export function createRulePacks(seed: KitRulePack[]): RulePacksState {
	// Copy so toggling never mutates the caller's fixtures.
	let packs = $state<KitRulePack[]>(seed.map((p) => ({ ...p })));
	const split = $derived(splitPacks(packs));

	return {
		get packs() {
			return packs;
		},
		get adopted() {
			return split.adopted;
		},
		get available() {
			return split.available;
		},
		isAdopted(id: string) {
			return packs.some((p) => p.id === id && p.adopted);
		},
		toggle(id: string) {
			packs = packs.map((p) => (p.id === id ? { ...p, adopted: !p.adopted } : p));
		}
	};
}
