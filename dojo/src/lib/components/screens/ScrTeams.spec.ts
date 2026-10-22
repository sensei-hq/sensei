import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrTeams from './ScrTeams.svelte';
import { toTeamMetricsView } from '$lib/team-metrics-view';

// The lead's team view (#307 R8, D38): each of the lead's teams, each member's key
// numbers. A lead on no team is told so; a failed read says it failed — neither is
// an empty table pretending to be data.

const view = toTeamMetricsView({
	since: '2026-10-16',
	days: 7,
	teams: [
		{
			id: 'tmA',
			name: 'Payments',
			members: [
				{
					principal_id: 'lena',
					display_name: 'Lena',
					email: null,
					you: true,
					metrics: { throughput: { total: 12, mean: 4, days: 3, repositories: 2, last_on: '2026-10-21' } }
				},
				{ principal_id: 'mo', display_name: 'Mo', email: null, you: false, metrics: {} }
			]
		}
	]
});

describe('ScrTeams', () => {
	afterEach(cleanup);

	it("lists each team with its members and their numbers", () => {
		const { getByText, getAllByText } = render(ScrTeams, { props: { orgName: 'Acme', view, error: null } });
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('Lena')).toBeTruthy();
		expect(getByText('Mo')).toBeTruthy();
		expect(getByText('12')).toBeTruthy();
		expect(getAllByText('Sessions').length).toBeGreaterThan(0);
		expect(getByText(/last 7 days/)).toBeTruthy();
	});

	it('a lead on no team is told so', () => {
		const { getByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: toTeamMetricsView({ since: '2026-10-16', days: 7, teams: [] }), error: null }
		});
		expect(getByText('You are on no team yet.')).toBeTruthy();
	});

	it('a failed read says so, and shows no table', () => {
		const { getByRole, queryByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: null, error: 'lead role required' }
		});
		expect(getByRole('alert').textContent).toContain('lead role required');
		expect(queryByText('You are on no team yet.')).toBeNull();
	});
});
