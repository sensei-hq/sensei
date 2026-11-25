// The org repository screen (#532): header (name, key, visibility,
// activity, branches), metric cards, and the diagrams link preserving the
// branch. Empty window reads empty; a failed read is an error state.
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import Page from './+page.svelte';

const BASE = {
	slug: 'acme',
	orgName: 'Acme',
	tenantKey: 'organization/acme',
	repo: {
		id: 'r1',
		key: 'github.com/acme/api',
		name: 'api',
		branches: [{ branch: 'main', is_default: true }],
		configurable: true,
		last_activity_at: new Date(Date.now() - 3 * 3_600_000).toISOString(),
		visibility: 'public'
	},
	branch: 'main',
	metricsView: {
		since: '2026-10-05',
		days: 30,
		repos: [
			{
				repository_id: 'r1',
				repo_key: 'github.com/acme/api',
				name: 'acme/api',
				last_activity_at: new Date(Date.now() - 3 * 3_600_000).toISOString(),
				cards: [{ key: 'throughput', label: 'Throughput', value: '4', points: [4], empty: false }]
			}
		]
	},
	metricsError: null
};

describe('the repository screen', () => {
	it('shows the header, the metric cards, and the diagrams link on the branch', () => {
		const { getByText, getAllByText, getByRole } = render(Page, { data: BASE } as never);
		expect(getByText('api')).toBeTruthy();
		expect(getByText(/github\.com\/acme\/api/)).toBeTruthy();
		// Header and card agree on the recency: one key, shown twice.
		expect(getAllByText(/Active 3h/)).toHaveLength(2);
		expect(getByText('Throughput')).toBeTruthy();
		const link = getByRole('link', { name: /diagrams/i });
		expect((link as HTMLAnchorElement).href).toContain('/org/acme/repositories/r1/diagrams/structure?branch=main');
	});

	it('a failed metrics read is an error state, never empty charts', () => {
		const { getByRole, queryByText } = render(Page, {
			data: { ...BASE, metricsView: null, metricsError: 'the request failed' }
		} as never);
		expect(getByRole('alert').textContent).toMatch(/the request failed/);
		expect(queryByText('Throughput')).toBeNull();
	});
});
