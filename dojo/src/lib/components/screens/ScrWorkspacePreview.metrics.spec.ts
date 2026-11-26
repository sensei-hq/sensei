// ScrWorkspacePreview — per-repository metrics cards show the recency they
// sort by (#531): the activity timestamp, never a mismatched one.
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import ScrWorkspacePreview from './ScrWorkspacePreview.svelte';

describe('ScrWorkspacePreview — metrics recency (#531)', () => {
	it('shows each repo’s activity beside its name', () => {
		const { getByText } = render(ScrWorkspacePreview, {
			props: {
				workspace: { id: 'w1', name: 'Act', repo: 'acme', classification: 'company', phase: 'adopt' },
				ladder: [],
				conflicts: [],
				metrics: {
					since: '2026-10-05',
					days: 30,
					repos: [
						{
							repository_id: 'r1',
							repo_key: 'github.com/acme/api',
							name: 'acme/api',
							last_activity_at: new Date(Date.now() - 2 * 3_600_000).toISOString(),
							cards: []
						}
					]
				}
			}
		});
		expect(getByText(/acme\/api/)).toBeTruthy();
		expect(getByText(/Active 2h/)).toBeTruthy();
	});
});
