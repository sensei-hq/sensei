// The `$state` rune store for the constitution library (Chunk 2). Holds the
// working selection — which catalog rules are included, per-rule level (via the
// pack) and per-rule non-negotiable ★, plus the "write your own" authored rules —
// and delegates ALL math to the pure helpers in library-view.ts. The component
// reads getters (counts, per-row flags) and calls the mutators (toggle, star,
// level, author) so the template stays presentational.
//
// One store instance per screen (`createLibraryStore()`), created in the page so
// state doesn't leak across renders or tests. Hard-guard rules are auto-locked
// non-negotiable by the view logic; the store never records a star for them, and
// `toggleStar` is a no-op on a hard-locked rule (the ★ can't be relaxed).

import {
	findRule,
	includedCount,
	isHardLocked,
	isIncluded,
	isNonNegotiable,
	isPackFullyChosen,
	levelForPack,
	nonNegotiableCount,
	packChosenCount,
	selectedCount,
	type AuthoredRule,
	type LibrarySelection
} from './library-view';
import type { LibAreaId, LibLevelId, LibPack } from './library-data';

export function createLibraryStore() {
	// The three selection maps + authored list, as reactive state. Kept as the
	// shape library-view.ts reasons over so the pure helpers apply directly.
	let included = $state<Record<string, boolean>>({});
	let starred = $state<Record<string, boolean>>({});
	let packLevel = $state<Record<string, LibLevelId>>({});
	let authored = $state<AuthoredRule[]>([]);
	// Monotonic id source for authored rules (stable keys for the #each).
	let nextAuthoredId = 0;

	/** A plain snapshot the pure helpers read (never mutated by them). */
	function selection(): LibrarySelection {
		return { included, starred, packLevel, authored };
	}

	return {
		// ── reads (delegated to library-view) ────────────────────────────────
		/** Total rules selected (included catalog rules + authored). */
		get selectedCount() {
			return selectedCount(selection());
		},
		/** Included catalog rules only (excludes authored). */
		get includedCount() {
			return includedCount(selection());
		},
		/** How many selected rules are non-negotiable (hard-locked + starred + authored hard). */
		get nonNegotiableCount() {
			return nonNegotiableCount(selection());
		},
		/** The authored "write your own" rules, in add order. */
		get authored(): readonly AuthoredRule[] {
			return authored;
		},

		/** Is this rule included in the constitution? */
		isIncluded(ruleId: string): boolean {
			return isIncluded(selection(), ruleId);
		},
		/** Is this rule effectively non-negotiable (hard-locked or starred, once included)? */
		isNonNegotiable(ruleId: string): boolean {
			return isNonNegotiable(selection(), ruleId);
		},
		/** The level a pack applies at (override or default). */
		levelForPack(packId: string): LibLevelId {
			return levelForPack(selection(), packId);
		},
		/** How many of a pack's rules are included. */
		packChosenCount(pack: LibPack): number {
			return packChosenCount(selection(), pack);
		},
		/** Are all of a pack's rules included (add-all ↔ clear-all)? */
		isPackFullyChosen(pack: LibPack): boolean {
			return isPackFullyChosen(selection(), pack);
		},

		// ── mutations ─────────────────────────────────────────────────────────
		/** Include ↔ exclude a single rule. */
		toggleRule(ruleId: string) {
			included = { ...included, [ruleId]: !included[ruleId] };
		},

		/** Include or clear every rule of a pack (whichever moves it toward the
		 *  opposite state — clear when fully chosen, else add all). */
		toggleAll(pack: LibPack) {
			const target = !isPackFullyChosen(selection(), pack);
			const next = { ...included };
			for (const rule of pack.rules) next[rule.id] = target;
			included = next;
		},

		/** Toggle the non-negotiable ★ for a rule. No-op on a hard-locked rule
		 *  (a hard guard can't be relaxed once included). */
		toggleStar(ruleId: string) {
			const rule = findRule(ruleId);
			if (rule && isHardLocked(rule, isIncluded(selection(), ruleId))) return;
			starred = { ...starred, [ruleId]: !starred[ruleId] };
		},

		/** Set the level a pack applies at. */
		setPackLevel(packId: string, level: LibLevelId) {
			packLevel = { ...packLevel, [packId]: level };
		},

		/** Add a user-authored rule, classified under an area at a level. Trims and
		 *  ignores blank text. Returns the new rule's id (or null when blank). */
		addAuthored(text: string, area: LibAreaId, level: LibLevelId, hard: boolean): string | null {
			const trimmed = text.trim();
			if (!trimmed) return null;
			const id = `authored:${nextAuthoredId++}`;
			authored = [...authored, { id, text: trimmed, area, level, hard }];
			return id;
		},

		/** Remove an authored rule by id. */
		removeAuthored(id: string) {
			authored = authored.filter((a) => a.id !== id);
		},

		/** Authored rules classified under one area (for the per-area list). */
		authoredInArea(area: LibAreaId): readonly AuthoredRule[] {
			return authored.filter((a) => a.area === area);
		}
	};
}

/** The store instance type (for typing props that receive it). */
export type LibraryStore = ReturnType<typeof createLibraryStore>;
