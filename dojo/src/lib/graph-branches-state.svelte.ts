// The tracked-branches state for one repository's diagrams (#268): the branch
// list with a draft to track and a two-step untrack — untracking deletes
// that branch's graph, so the screen confirms in-page first. Rendered only
// for whoever may configure the repository (`configurable_by_me`); the
// route and `set_graph_branch` re-decide anyway.
import { GRAPH_LABELS } from './graph-labels';
import type { GraphBranchWire, GraphBranchOutcomeWire } from './client-data';

export interface BranchDraft {
	branch: string;
	label: string;
}

export function emptyDraft(): BranchDraft {
	return { branch: '', label: '' };
}

export function draftProblem(d: BranchDraft): string | null {
	if (d.branch.trim() === '') return 'Enter the branch to track.';
	if (d.label !== '' && !(GRAPH_LABELS as readonly string[]).includes(d.label)) {
		return 'Label is production, testing, development, or none.';
	}
	return null;
}

export class GraphBranchesState {
	branches = $state<GraphBranchWire[]>([]);
	draft = $state<BranchDraft>(emptyDraft());
	problem = $state<string | null>(null);
	working = $state(false);
	error = $state<string | null>(null);
	pendingUntrack = $state<string | null>(null);

	constructor(
		private repoKey: string,
		initial: GraphBranchWire[],
		private write: (
			repoKey: string,
			branch: string,
			tracked: boolean,
			label: string | null
		) => Promise<GraphBranchOutcomeWire>
	) {
		this.branches = [...initial];
	}

	async track(): Promise<void> {
		const problem = draftProblem(this.draft);
		this.problem = problem;
		if (problem) return;
		this.working = true;
		this.error = null;
		try {
			const branch = this.draft.branch.trim();
			const out = await this.write(this.repoKey, branch, true, this.draft.label || null);
			const rest = this.branches.filter((b) => b.branch !== out.branch);
			this.branches = [...rest, { branch: out.branch, is_default: out.is_default }].sort((a, b) =>
				a.branch < b.branch ? -1 : a.branch > b.branch ? 1 : 0
			);
			this.draft = emptyDraft();
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'the request failed';
		} finally {
			this.working = false;
		}
	}

	askUntrack(branch: string): void {
		this.pendingUntrack = branch;
	}

	cancelUntrack(): void {
		this.pendingUntrack = null;
	}

	async confirmUntrack(): Promise<void> {
		const branch = this.pendingUntrack;
		if (!branch) return;
		this.working = true;
		this.error = null;
		try {
			await this.write(this.repoKey, branch, false, null);
			this.branches = this.branches.filter((b) => b.branch !== branch);
			this.pendingUntrack = null;
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'the request failed';
		} finally {
			this.working = false;
		}
	}
}
