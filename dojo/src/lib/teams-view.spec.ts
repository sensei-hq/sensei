import { describe, it, expect } from 'vitest';
import { toTeamViews, addableMembers } from './teams-view';

// A dōjō's teams (#307 D7) as the Teams tab shows them.
describe('teams view', () => {
	it('maps the wire rows', () => {
		expect(
			toTeamViews([
				{ id: 't1', name: 'Payments', members: [{ principal_id: 'u1', display_name: 'Ada', email: null }] }
			])
		).toEqual([{ id: 't1', name: 'Payments', members: [{ principalId: 'u1', name: 'Ada', email: null }] }]);
	});

	it("offers the dōjō's members not yet on the team", () => {
		const team = { id: 't1', name: 'P', members: [{ principalId: 'u1', name: 'Ada', email: null }] };
		const members = [
			{ name: 'Ada', userId: 'u1', git: '', role: 'admin', scopes: '', active: '' },
			{ name: 'Dev', userId: 'u2', git: '', role: 'contributor', scopes: '', active: '' },
			{ name: 'Fixture', git: '', role: 'contributor', scopes: '', active: '' }
		];
		expect(addableMembers(team, members)).toEqual([{ id: 'u2', name: 'Dev' }]);
	});
});
