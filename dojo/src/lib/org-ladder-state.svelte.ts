// The reactive authoring state for ScrOrgLadder (mockup) — the maintainer's
// dōjō-constitution editor. Holds the four bits of UI state as `$state` (the
// active section, the per-rule include map, the show-excluded toggle, and the
// RuleEditor open/close) and delegates ALL grouping + include math to the pure
// `org-ladder-view` module, so the screen stays presentational. Kept as a
// `.svelte.ts` rune module (like `preview/state.svelte.ts`).

import type { KitConstitutionSection, KitRule } from './components/kit/types';
import {
	excludedCount,
	includeKey,
	isIncluded,
	sectionsByGroup,
	type LadderGroup
} from './org-ladder-view';

/** The RuleEditor target — a fresh rule (no `rule`) or an existing one to edit. */
export interface EditorTarget {
	/** The rule being edited; absent ⇒ adding a new rule. */
	rule?: KitRule;
}

/** The reactive authoring state a ScrOrgLadder binds to. */
export interface OrgLadderState {
	/** The sections grouped Company · Teams · Stacks (the left rail). */
	readonly groups: LadderGroup[];
	/** The focused section id. */
	readonly active: string;
	/** The focused section object. */
	readonly section: KitConstitutionSection;
	/** How many of the active section's rules are excluded. */
	readonly excluded: number;
	/** Whether excluded rules are revealed. */
	readonly showExcluded: boolean;
	/** The open editor target, or null when closed. */
	readonly editing: EditorTarget | null;
	/** Whether a rule (by index, in the active section) is included. */
	isIncluded(index: number): boolean;
	/** Focus a section. */
	setActive(id: string): void;
	/** Toggle a rule's include state (active section, by index). */
	toggleInclude(index: number): void;
	/** Toggle the excluded-rules reveal. */
	toggleShowExcluded(): void;
	/** Open the editor to add a new rule. */
	openNew(): void;
	/** Open the editor to edit an existing rule. */
	openEdit(rule: KitRule): void;
	/** Close the editor. */
	closeEditor(): void;
}

/** Build the authoring state for a dōjō's constitution sections. */
export function createOrgLadder(sections: KitConstitutionSection[]): OrgLadderState {
	const groups = sectionsByGroup(sections);

	let active = $state(sections[0]?.id ?? '');
	let include = $state<Record<string, boolean>>({});
	let showExcluded = $state(false);
	let editing = $state<EditorTarget | null>(null);

	const section = $derived(sections.find((s) => s.id === active) ?? sections[0]);
	const excluded = $derived(excludedCount(include, section));

	return {
		groups,
		get active() {
			return active;
		},
		get section() {
			return section;
		},
		get excluded() {
			return excluded;
		},
		get showExcluded() {
			return showExcluded;
		},
		get editing() {
			return editing;
		},
		isIncluded(index: number) {
			return isIncluded(include, active, index);
		},
		setActive(id: string) {
			active = id;
			showExcluded = false;
		},
		toggleInclude(index: number) {
			const key = includeKey(active, index);
			include = { ...include, [key]: !isIncluded(include, active, index) };
		},
		toggleShowExcluded() {
			showExcluded = !showExcluded;
		},
		openNew() {
			editing = {};
		},
		openEdit(rule: KitRule) {
			editing = { rule };
		},
		closeEditor() {
			editing = null;
		}
	};
}
