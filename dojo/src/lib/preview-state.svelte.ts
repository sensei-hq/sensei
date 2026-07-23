// The `$state` rune store for the effective-constitution preview (Chunk 3). Holds
// the two pieces of UI state — which sample project is selected and an optional
// classification override (company↔client) — and delegates ALL resolution to the
// pure engine in preview-view.ts (via the ladder builder in preview-data.ts). The
// component reads getters (the current project, the composed constitution, the
// counts, whether the classification is overridden) and calls the mutators
// (select, reclassify, reset) so the template stays presentational.
//
// One store instance per screen (`createPreviewStore()`), created in the page so
// state doesn't leak across renders or tests. The override is per-project:
// selecting a different project drops it (its own base classification applies).

import { buildLadder, projectById, type PreviewProject, type ProjectKind } from './preview-data';
import { resolveConstitution, type EffectiveConstitution } from './preview-view';

export function createPreviewStore(initial = 'globex') {
	// The selected sample project id, and a per-project classification override
	// (company↔client). `null` override → the project's base kind applies.
	let projectId = $state(projectById(initial).id);
	let override = $state<ProjectKind | null>(null);

	/** The selected base project (before any classification override). */
	const baseProject = $derived(projectById(projectId));

	/** The effective classification — the override if set, else the base kind. */
	const effectiveKind = $derived<ProjectKind>(override ?? baseProject.kind);

	/** The project as resolved — base project with the effective kind applied.
	 *  When reclassified to company, the client list is dropped (no client rungs);
	 *  reclassifying to client keeps any bound clients. This mirrors the mockup's
	 *  override handling (dojo-preview.jsx `proj`). */
	const project = $derived<PreviewProject>(
		override && override !== baseProject.kind
			? {
					...baseProject,
					kind: override,
					clients: override === 'client' ? baseProject.clients : undefined
				}
			: baseProject
	);

	/** The composed effective constitution for the current project (ladder +
	 *  conflicts + counts), recomputed whenever the project or override changes. */
	const constitution = $derived<EffectiveConstitution>(resolveConstitution(buildLadder(project)));

	return {
		/** The resolved project (base kind or the override applied). */
		get project(): PreviewProject {
			return project;
		},
		/** The base (un-overridden) project — for the picker + "not right?" prompt. */
		get baseProject(): PreviewProject {
			return baseProject;
		},
		/** The effective classification (override or base). */
		get effectiveKind(): ProjectKind {
			return effectiveKind;
		},
		/** Is the classification currently overridden from the project's base kind? */
		get isOverridden(): boolean {
			return override !== null && override !== baseProject.kind;
		},
		/** The composed constitution (ladder, conflicts, counts). */
		get constitution(): EffectiveConstitution {
			return constitution;
		},

		/** Select a different sample project; drops any classification override. */
		select(id: string) {
			projectId = projectById(id).id;
			override = null;
		},
		/** Reclassify the current project (company↔client). */
		reclassify(kind: ProjectKind) {
			override = kind;
		},
		/** Clear the classification override, back to the base kind. */
		resetClassification() {
			override = null;
		}
	};
}

/** The store instance type (for typing props that receive it). */
export type PreviewStore = ReturnType<typeof createPreviewStore>;
