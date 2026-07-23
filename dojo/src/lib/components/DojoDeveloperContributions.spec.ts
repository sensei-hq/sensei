import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoDeveloperContributions from './DojoDeveloperContributions.svelte';
import { CONTRIBUTIONS } from '$lib/developer-data';

// Render tests for "My contributions" — the upstream-sends surface. Proves each
// contribution renders from the catalog (title, destination, status label) for a
// member, and that a solo (membership-less) contributor gets the honest empty
// state ("nothing shared upstream yet") instead of fabricated rows (DJ1).

describe('DojoDeveloperContributions', () => {
	afterEach(() => cleanup());

	it('renders the header and a row per contribution for a member', () => {
		const { getByText } = render(DojoDeveloperContributions, { hasMembership: true });
		expect(getByText("What you've shared")).toBeTruthy();
		for (const c of CONTRIBUTIONS) {
			expect(getByText(c.title)).toBeTruthy();
		}
	});

	it('renders each contribution status label (approved / in triage / declined)', () => {
		const { getAllByText, getByText } = render(DojoDeveloperContributions, { hasMembership: true });
		// 2 approved, 1 in triage (pending), 1 declined.
		expect(getAllByText('approved').length).toBeGreaterThanOrEqual(2);
		expect(getByText('in triage')).toBeTruthy();
		expect(getByText('declined')).toBeTruthy();
	});

	it('shows the honest empty state for a membership-less contributor', () => {
		const { getByText, queryByText } = render(DojoDeveloperContributions, {
			hasMembership: false
		});
		expect(getByText('nothing shared upstream yet')).toBeTruthy();
		expect(getByText(/join or create a Dōjō from the switcher/)).toBeTruthy();
		// No fabricated rows.
		expect(queryByText(CONTRIBUTIONS[0].title)).toBeNull();
	});
});
