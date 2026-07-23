// The reactive drill-in state for ScrProjectPreview (mockup) — the selected
// rung (`active`) and the by-layer / consolidated `view` toggle. Delegates ALL
// resolution to the pure `dojo2-preview-view` module; here we only hold the two
// bits of UI state as `$state` so the screen stays presentational. Kept as a
// `.svelte.ts` rune module (like `preview-state.svelte.ts` and
// `your-work-view.svelte.ts`); the pure helpers stay unit-testable without a
// component.

import type { KitConflict, KitLadderRung, KitProject } from './components/kit/types';
import {
	discardedTexts,
	effectiveRules,
	lockCount,
	previewRungs,
	showsConflicts,
	type EffectiveRule
} from './dojo2-preview-view';

/** The by-layer (the ladder rungs) or consolidated (flat rules) view. */
export type PreviewView = 'layer' | 'consolidated';

/** The reactive drill-in state a ScrProjectPreview binds to. */
export interface ProjectPreviewState {
	/** The composed ladder rungs, broad → specific. */
	readonly rungs: KitLadderRung[];
	/** The consolidated constitution (rung rules minus discards). */
	readonly effective: EffectiveRule[];
	/** The losing-rule copy the ladder discarded. */
	readonly discarded: string[];
	/** The count of non-negotiable (★) rules across the rungs. */
	readonly locks: number;
	/** Whether the ladder shows its discarded conflicts. */
	readonly showConflicts: boolean;
	/** The focused rung id (`project` on open). */
	readonly active: string;
	/** The current view (`layer` on open). */
	readonly view: PreviewView;
	/** Focus a rung. */
	setActive(id: string): void;
	/** Switch the view. */
	setView(view: PreviewView): void;
	/** From the consolidated view, jump to the rung owning a scope level and
	 *  return to the layer view (mockup `onJump`). */
	jumpTo(level: string): void;
}

/** Build the drill-in state for a project against the kit ladder + conflicts. */
export function createProjectPreview(
	project: KitProject,
	ladder: KitLadderRung[],
	conflicts: KitConflict[]
): ProjectPreviewState {
	const rungs = previewRungs(project, ladder);
	const effective = effectiveRules(project, ladder, conflicts);
	const discarded = discardedTexts(project, conflicts);
	const locks = lockCount(rungs);
	const showConflicts = showsConflicts(project);

	let active = $state('project');
	let view = $state<PreviewView>('layer');

	return {
		rungs,
		effective,
		discarded,
		locks,
		showConflicts,
		get active() {
			return active;
		},
		get view() {
			return view;
		},
		setActive(id: string) {
			active = id;
		},
		setView(next: PreviewView) {
			view = next;
		},
		jumpTo(level: string) {
			const hit = rungs.find((r) => r.scope === level);
			if (hit) {
				active = hit.id;
				view = 'layer';
			}
		}
	};
}
