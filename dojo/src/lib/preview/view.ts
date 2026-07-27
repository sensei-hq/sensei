// Pure derivations behind the dojo project-constitution preview (mockup
// ScrProjectPreview). Ported faithfully from dojo2-app.jsx (`previewRungs` ·
// the `showConflicts` gate · the `effective` flatten · the lock/discard counts)
// but resolving off the kit fixtures the mockup used (`ladder` · `conflicts`).
//
// This is deliberately SEPARATE from the shipped `preview-view.ts` (which the
// (console) group's deeper resolution engine owns): the mockup's model is the
// simpler one — a project's classification picks which rung ids compose, the
// ladder discards the losing side of each fixture conflict, and the
// consolidated view is those rungs' rules minus the discarded copy. Pure over
// plain data so it unit-tests without a DOM.

import type { KitConflict, KitLadderRung, KitProject, KitRule } from '../components/kit/types';

/** A rule on the consolidated constitution — a rung rule with the scope it
 *  entered from attached (drives the "by layer" jump + the level chip). */
export type EffectiveRule = KitRule & { level: string };

/** The rung ids a classification composes, broad → specific (mockup
 *  `previewRungs`): a personal project stands on its own ladder; a client
 *  engagement layers the company baseline + the client rung under it; anything
 *  else (company/community) layers the company baseline. */
export function scopeIdsFor(classification: string): string[] {
	if (classification === 'personal') return ['personal', 'project', 'stack'];
	if (classification === 'client') return ['company', 'client', 'personal', 'project', 'stack'];
	return ['company', 'personal', 'project', 'stack'];
}

/** The ladder rungs that compose a project's constitution, in broad → specific
 *  order. Rungs the fixtures define carry their rules; a scope with no fixture
 *  rung (project · stack) synthesizes a stub so the ladder is always complete.
 *  The `project` rung is relabelled to the project's own name (mockup). */
export function previewRungs(project: KitProject, ladder: KitLadderRung[]): KitLadderRung[] {
	const byId = new Map(ladder.map((r) => [r.id, r]));
	return scopeIdsFor(project.classification).map((id) => {
		const rung = byId.get(id);
		if (rung) return id === 'project' ? { ...rung, name: project.name } : rung;
		// A scope the fixtures don't define — synthesize an empty rung so the
		// ladder still shows it (honest empty rather than a gap).
		return synthRung(id, project);
	});
}

/** Whether the ladder shows its discarded conflicts — only non-personal work
 *  layers multiple authorities that can collide (a personal project stands on
 *  its ladder alone). Mockup `showConflicts`. */
export function showsConflicts(project: KitProject): boolean {
	return project.classification !== 'personal';
}

/** The losing-rule copy the ladder discarded for a project — every fixture
 *  conflict's loser when conflicts show, else nothing. Mockup `discarded`. */
export function discardedTexts(project: KitProject, conflicts: KitConflict[]): string[] {
	return showsConflicts(project) ? conflicts.map((c) => c.loser.text) : [];
}

/** The consolidated constitution — every composed rung rule with its scope
 *  level attached, minus the rules the ladder discarded. Mockup `effective`. */
export function effectiveRules(
	project: KitProject,
	ladder: KitLadderRung[],
	conflicts: KitConflict[]
): EffectiveRule[] {
	const discarded = new Set(discardedTexts(project, conflicts));
	return previewRungs(project, ladder)
		.flatMap((rung) => (rung.rules ?? []).map((rule) => ({ ...rule, level: rung.scope })))
		.filter((rule) => !discarded.has(rule.text));
}

/** The count of non-negotiable (★/hard) rules across the composed rungs. */
export function lockCount(rungs: KitLadderRung[]): number {
	return rungs.reduce((n, r) => n + (r.rules ?? []).filter((x) => x.hard).length, 0);
}

// ── internals ────────────────────────────────────────────────────────────────

/** Human scope label + kanji + caption for a synthesized rung (a scope the
 *  fixtures don't carry). Keeps the ladder complete without fabricating rules. */
function synthRung(id: string, project: KitProject): KitLadderRung {
	switch (id) {
		case 'project':
			return {
				id,
				kanji: '件',
				scope: 'Project',
				name: project.name,
				caption: 'this repository · most specific',
				rules: []
			};
		case 'stack':
			return {
				id,
				kanji: '技',
				scope: 'Stack',
				name: 'toolchain',
				caption: 'the stack this project runs on',
				rules: []
			};
		default:
			return { id, kanji: '·', scope: id, name: id, caption: '', rules: [] };
	}
}
