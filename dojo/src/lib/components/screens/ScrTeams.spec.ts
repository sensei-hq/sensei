import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrTeams from './ScrTeams.svelte';
import { toTeamMetricsView } from '$lib/team-metrics-view';
import type { TeamMetricsWire } from '$lib/team-metrics';

// The Teams view (#307 R8, D38): every member sees their teams' totals; a lead or
// an admin also sees each member's numbers. No team is said in words; a failed read
// says it failed — neither is an empty table pretending to be data.

const wire = (per_person: boolean): TeamMetricsWire => ({
	since: '2026-10-16',
	days: 7,
	per_person,
	k_min: 3,
	teams: [
		{
			id: 'tmA',
			name: 'Payments',
			member_count: 3,
			totals: { throughput: { total: 30, weighted: null, contributors: 3 } },
			members: per_person
				? [
						{
							principal_id: 'lena',
							display_name: 'Lena',
							email: null,
							you: true,
							metrics: { throughput: { total: 12, weighted: null, days: 3, repositories: 2, last_on: '2026-10-21' } }
						},
						{ principal_id: 'mo', display_name: 'Mo', email: null, you: false, metrics: {} }
					]
				: []
		}
	]
});

describe('ScrTeams', () => {
	afterEach(cleanup);

	it('a lead sees the team total and each member with their numbers', () => {
		const { getByText, getAllByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: toTeamMetricsView(wire(true)), error: null }
		});
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('30')).toBeTruthy();
		expect(getByText('Lena')).toBeTruthy();
		expect(getByText('Mo')).toBeTruthy();
		expect(getByText('12')).toBeTruthy();
		expect(getAllByText('Sessions').length).toBeGreaterThan(0);
		expect(getByText(/last 7 days/)).toBeTruthy();
	});

	it('below lead: the team total, and no person', () => {
		const { getByText, queryByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: toTeamMetricsView(wire(false)), error: null }
		});
		expect(getByText('30')).toBeTruthy();
		expect(queryByText('Lena')).toBeNull();
		expect(getByText(/at least 3 people/)).toBeTruthy();
	});

	it('on no team: told so', () => {
		const { getByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: toTeamMetricsView({ ...wire(true), teams: [] }), error: null }
		});
		expect(getByText('You are on no team yet.')).toBeTruthy();
	});

	it('a failed read says so, and shows no table', () => {
		const { getByRole, queryByText } = render(ScrTeams, {
			props: { orgName: 'Acme', view: null, error: 'not a member of this dōjō' }
		});
		expect(getByRole('alert').textContent).toContain('not a member of this dōjō');
		expect(queryByText('You are on no team yet.')).toBeNull();
	});
});
