// Unit tests for the tracked-branches state (`graph-branches-state.svelte.ts`,
// #268): validating a branch draft, tracking it, and untracking with an
// in-page confirm — untracking deletes that branch's graph. The screen
// renders this state only for whoever may configure the repository.
import { describe, it, expect, vi } from 'vitest';
import { GraphBranchesState, draftProblem, emptyDraft } from '$lib/graph-branches-state.svelte';

describe('branch draft rules', () => {
	it('names the problem with a blank branch', () => {
		expect(draftProblem({ ...emptyDraft(), branch: '  ' })).toBe('Enter the branch to track.');
		expect(draftProblem({ ...emptyDraft(), branch: 'develop' })).toBeNull();
	});

	it('a label outside the known set is refused', () => {
		expect(draftProblem({ ...emptyDraft(), branch: 'x', label: 'nope' })).toBe(
			'Label is production, testing, development, or none.'
		);
		expect(draftProblem({ ...emptyDraft(), branch: 'x', label: 'production' })).toBeNull();
	});
});

describe('GraphBranchesState', () => {
	const branches = [{ branch: 'main', is_default: true }];

	it('tracks the draft and merges the outcome into the list', async () => {
		const write = vi.fn(async () => ({ repo_key: 'k', branch: 'dev', tracked: true, is_default: false }));
		const s = new GraphBranchesState('k', branches, write);
		s.draft = { branch: 'dev', label: '' };
		await s.track();
		expect(write).toHaveBeenCalledWith('k', 'dev', true, null);
		expect(s.branches).toEqual([
			{ branch: 'dev', is_default: false },
			{ branch: 'main', is_default: true }
		]);
		expect(s.draft).toEqual(emptyDraft());
	});

	it('does not track an invalid draft', async () => {
		const write = vi.fn();
		const s = new GraphBranchesState('k', branches, write);
		s.draft = { branch: '  ', label: '' };
		await s.track();
		expect(write).not.toHaveBeenCalled();
		expect(s.problem).toBe('Enter the branch to track.');
	});

	it('untracking arms first and deletes on confirm', async () => {
		const write = vi.fn(async () => ({ repo_key: 'k', branch: 'dev', tracked: false, is_default: false }));
		const s = new GraphBranchesState(
			'k',
			[...branches, { branch: 'dev', is_default: false }],
			write
		);
		s.askUntrack('dev');
		expect(s.pendingUntrack).toBe('dev');
		expect(write).not.toHaveBeenCalled();
		await s.confirmUntrack();
		expect(write).toHaveBeenCalledWith('k', 'dev', false, null);
		expect(s.branches).toEqual([{ branch: 'main', is_default: true }]);
		expect(s.pendingUntrack).toBeNull();
	});

	it('a failed write is carried as the error, the list untouched', async () => {
		const write = vi.fn(async () => {
			throw new Error('forbidden');
		});
		const s = new GraphBranchesState('k', branches, write);
		s.draft = { branch: 'dev', label: '' };
		await s.track();
		expect(s.error).toBe('forbidden');
		expect(s.branches).toEqual(branches);
	});
});
