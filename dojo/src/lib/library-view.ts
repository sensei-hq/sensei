// Pure selection logic for the constitution library (Chunk 2). All the math the
// screen needs — which rules are included, the per-rule level, the per-rule
// non-negotiable ★ (with hard guards auto-locked), the authored "write your own"
// rules, and the live counts — lives here as side-effect-free functions over
// plain data structures, so it unit-tests without a DOM and the `$state` store
// (library-state.svelte.ts) and the component stay thin.
//
// The store owns three maps + the authored list; this module reads them and the
// catalog (library-data.ts) to answer: is a rule effectively non-negotiable?
// what level applies? how many rules are selected, and how many are
// non-negotiable? The hard-lock rule (a hard-guard rule is non-negotiable the
// moment it's included, and can't be relaxed) is enforced here so both the row
// UI and the counts agree.

import { ALL_PACKS, LIB_LEVELS, type LibLevelId, type LibPack, type LibRule } from './library-data';

/** The working selection the store holds and this module reasons over. Maps are
 *  keyed by the stable rule id (library-data `LibRule.id`); the pack-level map is
 *  keyed by pack id. `authored` are the user's free-text "write your own" rules. */
export interface LibrarySelection {
	/** rule id → included in the constitution. */
	included: Record<string, boolean>;
	/** rule id → user-set non-negotiable ★ (ignored for hard-guard rules, which
	 *  are always non-negotiable once included). */
	starred: Record<string, boolean>;
	/** pack id → the level chosen for that pack (defaults to the pack's defLevel). */
	packLevel: Record<string, LibLevelId>;
	/** free-text rules the user authored, in add order. */
	authored: readonly AuthoredRule[];
}

/** A user-authored rule — classified under an area, at a level, optionally ★. */
export interface AuthoredRule {
	/** stable id (a monotonically increasing counter from the store). */
	id: string;
	text: string;
	area: string;
	level: LibLevelId;
	hard: boolean;
}

/** An empty selection (nothing included, no overrides, no authored rules). */
export function emptySelection(): LibrarySelection {
	return { included: {}, starred: {}, packLevel: {}, authored: [] };
}

// ── catalog helpers ──────────────────────────────────────────────────────────

/** Packs in one area (empty list for an unknown area id). */
export function packsInArea(area: string): readonly LibPack[] {
	return ALL_PACKS.filter((p) => p.area === area);
}

/** The rule with this id anywhere in the catalog, or undefined. */
export function findRule(ruleId: string): LibRule | undefined {
	for (const pack of ALL_PACKS) {
		const rule = pack.rules.find((r) => r.id === ruleId);
		if (rule) return rule;
	}
	return undefined;
}

/** The default level a pack applies at (its `defLevel`), used when the selection
 *  has no explicit override for it. */
export function defaultLevel(packId: string): LibLevelId {
	return ALL_PACKS.find((p) => p.id === packId)?.defLevel ?? 'org';
}

// ── per-rule / per-pack resolution ───────────────────────────────────────────

/** Is this rule currently included in the constitution? */
export function isIncluded(sel: LibrarySelection, ruleId: string): boolean {
	return !!sel.included[ruleId];
}

/** The level applied to a pack — the explicit override if set, else its default. */
export function levelForPack(sel: LibrarySelection, packId: string): LibLevelId {
	return sel.packLevel[packId] ?? defaultLevel(packId);
}

/** A hard-guard rule can't be relaxed: once included it's locked non-negotiable.
 *  This is the single source of truth the row (★ locked) and the counts share. */
export function isHardLocked(rule: Pick<LibRule, 'hard'>, included: boolean): boolean {
	return included && rule.hard;
}

/**
 * Is a rule effectively non-negotiable? True when it's included AND either a hard
 * guard (auto-locked) or the user starred it. A rule that isn't included is never
 * non-negotiable (the ★ only counts once the rule is in the constitution).
 */
export function isNonNegotiable(sel: LibrarySelection, ruleId: string): boolean {
	if (!isIncluded(sel, ruleId)) return false;
	const rule = findRule(ruleId);
	if (rule?.hard) return true;
	return !!sel.starred[ruleId];
}

// ── counts (the live footer) ─────────────────────────────────────────────────

/** How many catalog rules are currently included. */
export function includedCount(sel: LibrarySelection): number {
	return Object.values(sel.included).filter(Boolean).length;
}

/** Total rules selected = included catalog rules + authored rules (the footer's
 *  headline "N rules selected"). */
export function selectedCount(sel: LibrarySelection): number {
	return includedCount(sel) + sel.authored.length;
}

/** How many selected rules are non-negotiable — hard-locked or starred catalog
 *  rules plus authored rules marked hard (the footer's "★ M non-negotiable"). */
export function nonNegotiableCount(sel: LibrarySelection): number {
	const catalog = Object.entries(sel.included).filter(
		([ruleId, on]) => on && isNonNegotiable(sel, ruleId)
	).length;
	const authored = sel.authored.filter((a) => a.hard).length;
	return catalog + authored;
}

// ── whole-pack helpers (the "add all / clear all" toggle) ────────────────────

/** How many rules of a pack are currently included. */
export function packChosenCount(sel: LibrarySelection, pack: LibPack): number {
	return pack.rules.filter((r) => isIncluded(sel, r.id)).length;
}

/** Is every rule of a pack included (drives the "add all ↔ clear all" label)? */
export function isPackFullyChosen(sel: LibrarySelection, pack: LibPack): boolean {
	return pack.rules.length > 0 && packChosenCount(sel, pack) === pack.rules.length;
}

// ── level metadata (for the pills) ───────────────────────────────────────────

/** The human label for a level id (`org` → "Org"), falling back to the id. */
export function levelLabel(level: LibLevelId): string {
	return LIB_LEVELS.find((l) => l.id === level)?.label ?? level;
}
