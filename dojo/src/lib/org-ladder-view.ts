// Pure derivations behind the dōjō-authoring ladder (mockup ScrOrgLadder) —
// the maintainer surface where a dōjō authors its OWN constitution by scope
// (company-wide · per team · per stack). Ported faithfully from dojo2-app.jsx
// (`groups` · the `inc` include map · `excluded` · `RULE_FAMILIES`). Pure over
// plain data so the grouping, per-rule include state, and editor vocab
// unit-test without a component, keeping ScrOrgLadder presentational.

import type { KitConstitutionSection } from './components/kit/types';

/** The authoring groups, broad → specific (mockup `groups`). A dōjō authors at
 *  the company scope, per team, and per stack. */
export const LADDER_GROUPS = ['Company', 'Teams', 'Stacks'] as const;

/** A rule family the editor picks from — a brand glyph + its human label
 *  (mockup `RULE_FAMILIES`). */
export interface RuleFamily {
	/** The brand glyph stored on the rule (`kanji`). */
	kanji: string;
	label: string;
}

/** The six rule families a new/edited rule can carry (mockup `RULE_FAMILIES`). */
export const RULE_FAMILIES: RuleFamily[] = [
	{ kanji: '守', label: 'guard' },
	{ kanji: '紋', label: 'pattern' },
	{ kanji: '理', label: 'principle' },
	{ kanji: '検', label: 'review' },
	{ kanji: '技', label: 'stack' },
	{ kanji: '盾', label: 'shield' }
];

/** The default family glyph — a guard (守), matching the editor's initial state. */
export const DEFAULT_FAMILY = '守';

/** A named authoring group with the sections that sit under it. */
export interface LadderGroup {
	group: string;
	sections: KitConstitutionSection[];
}

/** Bucket a dōjō's constitution sections under their authoring group, in the
 *  canonical Company · Teams · Stacks order, dropping any group with no
 *  sections (mockup's `groups.map(... items.length ? ...)`). */
export function sectionsByGroup(sections: KitConstitutionSection[]): LadderGroup[] {
	return LADDER_GROUPS.map((group) => ({
		group,
		sections: sections.filter((s) => s.group === group)
	})).filter((g) => g.sections.length > 0);
}

/** The stable per-rule include-map key — section id + rule index (mockup
 *  `active + i`). */
export function includeKey(sectionId: string, index: number): string {
	return sectionId + ':' + index;
}

/** Whether a rule is included — included unless its key is explicitly `false`,
 *  so a fresh (empty) map includes every rule (mockup `isIn`). */
export function isIncluded(
	include: Record<string, boolean>,
	sectionId: string,
	index: number
): boolean {
	return include[includeKey(sectionId, index)] !== false;
}

/** How many of a section's rules are currently excluded (mockup `excluded`). */
export function excludedCount(
	include: Record<string, boolean>,
	section: KitConstitutionSection
): number {
	return (section.rules ?? []).filter((_, i) => !isIncluded(include, section.id, i)).length;
}

/** Resolve a family glyph, defaulting an unknown/absent family to a guard (守). */
export function familyKanji(kanji: string | null | undefined): string {
	return kanji && RULE_FAMILIES.some((f) => f.kanji === kanji) ? kanji : DEFAULT_FAMILY;
}
