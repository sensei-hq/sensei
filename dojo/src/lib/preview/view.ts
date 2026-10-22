// Pure derivations behind the dojo workspace-constitution preview (mockup
// ScrWorkspacePreview). Ported faithfully from dojo2-app.jsx (`previewRungs` ·
// the `showConflicts` gate · the `effective` flatten · the lock/discard counts)
// but resolving off the kit fixtures the mockup used (`ladder` · `conflicts`).
//
// This is deliberately SEPARATE from the shipped `preview-view.ts` (which the
// (console) group's deeper resolution engine owns): the mockup's model is the
// simpler one — the daemon composed the ladder (it owns scoping), so we render
// every rung it resolved broad→specific, the ladder discards the losing side of
// each conflict, and the consolidated view is those rungs' rules minus the
// discarded copy. `classification` only gates whether conflicts show. Pure over
// plain data so it unit-tests without a DOM.

import type { KitConflict, KitLadderRung, KitWorkspace, KitRule } from '../components/kit/types';

/** A rule on the consolidated constitution — a rung rule with the scope it
 *  entered from attached (drives the "by layer" jump + the level chip). */
export type EffectiveRule = KitRule & { level: string };

/** The ladder rungs that compose a workspace's constitution, broad → specific.
 *  The daemon OWNS scoping — it resolves exactly the scopes that apply to this
 *  repo (memories + adopted packs at their adoption scope) — so the dōjō renders
 *  that ladder faithfully and never re-filters by classification. Re-filtering
 *  (the old mockup scaffold) silently dropped every composed scope whose key
 *  wasn't one of a hardcoded few (e.g. `general`, `organization`, `team`), so a
 *  real 30-rule constitution showed as one rule. The workspace rung (the daemon's
 *  scope key `workspace`) is relabelled to the workspace's own name; when the daemon
 *  composed no workspace-scoped rung, an empty one is synthesized as the most-specific anchor so the ladder always ends
 *  at the repository (honest empty, never a fabricated rule). */
export function previewRungs(workspace: KitWorkspace, ladder: KitLadderRung[]): KitLadderRung[] {
	const rungs = ladder.map((r) => (r.id === 'workspace' ? { ...r, name: workspace.name } : r));
	if (!rungs.some((r) => r.id === 'workspace')) rungs.push(workspaceAnchor(workspace));
	return rungs;
}

/** Whether the ladder shows its discarded conflicts — only non-personal work
 *  layers multiple authorities that can collide (a personal workspace stands on
 *  its ladder alone). Mockup `showConflicts`. */
export function showsConflicts(workspace: KitWorkspace): boolean {
	return workspace.classification !== 'personal';
}

/** The losing-rule copy the ladder discarded for a workspace — every fixture
 *  conflict's loser when conflicts show, else nothing. Mockup `discarded`. */
export function discardedTexts(workspace: KitWorkspace, conflicts: KitConflict[]): string[] {
	return showsConflicts(workspace) ? conflicts.map((c) => c.loser.text) : [];
}

/** The consolidated constitution — every composed rung rule with its scope
 *  level attached, minus the rules the ladder discarded. Mockup `effective`. */
export function effectiveRules(
	workspace: KitWorkspace,
	ladder: KitLadderRung[],
	conflicts: KitConflict[]
): EffectiveRule[] {
	const discarded = new Set(discardedTexts(workspace, conflicts));
	return previewRungs(workspace, ladder)
		.flatMap((rung) => (rung.rules ?? []).map((rule) => ({ ...rule, level: rung.scope })))
		.filter((rule) => !discarded.has(rule.text));
}

/** The count of non-negotiable (★/hard) rules across the composed rungs. */
export function lockCount(rungs: KitLadderRung[]): number {
	return rungs.reduce((n, r) => n + (r.rules ?? []).filter((x) => x.hard).length, 0);
}

// ── internals ────────────────────────────────────────────────────────────────

/** The most-specific "this repository" anchor rung — synthesized empty when the
 *  daemon composed no workspace-scoped rule, so the ladder always ends at the repo
 *  (honest empty, never a fabricated rule). */
function workspaceAnchor(workspace: KitWorkspace): KitLadderRung {
	return {
		id: 'workspace',
		kanji: '件',
		scope: 'Workspace',
		name: workspace.name,
		caption: 'this repository · most specific',
		rules: []
	};
}
