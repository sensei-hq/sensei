import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoDeveloperTeams from './DojoDeveloperTeams.svelte';
import { orgs } from '$lib/dojo-data';

// Render tests for "My teams" — the developer console's memberships surface.
// Proves each membership row renders from the injected memberships (name, role,
// what it follows), that the client-anonymization note appears when there's a
// client Dōjō, and that a solo (membership-less) contributor gets the honest
// empty state instead of fabricated rows (DJ1 — a personal surface, no join-gate).

describe('DojoDeveloperTeams', () => {
	afterEach(() => cleanup());

	it('renders the header and a row per membership', () => {
		const { getByText, getAllByText } = render(DojoDeveloperTeams, { memberships: orgs });
		expect(getByText('Your teams & orgs')).toBeTruthy();
		expect(getByText('Acme Corp')).toBeTruthy();
		expect(getByText('Rust Guild')).toBeTruthy();
		// The "Personal" org name + its uppercase kind label both render (name row
		// + kind caption), so assert on the pair rather than a single match.
		expect(getAllByText('Personal').length).toBe(2);
		// The count chip pluralizes.
		expect(getByText(`${orgs.length} memberships`)).toBeTruthy();
	});

	it('shows the role and what each membership follows', () => {
		const { getByText } = render(DojoDeveloperTeams, { memberships: orgs });
		// Employer → Contributor; Personal → Owner (roleForKind overlay).
		expect(getByText('Owner')).toBeTruthy();
		// acme follows Web · Auth · Payments (followsForMembership overlay).
		expect(getByText('Web · Auth · Payments')).toBeTruthy();
	});

	it('surfaces the client-anonymization note when a client membership is present', () => {
		const { getByText } = render(DojoDeveloperTeams, { memberships: orgs });
		expect(getByText(/automatically anonymized/)).toBeTruthy();
	});

	it('hides the anonymization note when there is no client membership', () => {
		const personalOnly = orgs.filter((o) => o.kind === 'Personal');
		const { queryByText } = render(DojoDeveloperTeams, { memberships: personalOnly });
		expect(queryByText(/automatically anonymized/)).toBeNull();
	});

	it('shows the honest empty state for a membership-less (solo) contributor', () => {
		const { getByText, queryByText } = render(DojoDeveloperTeams, { memberships: [] });
		// DojoJoinEmpty copy, scoped to "your teams" — no fabricated rows.
		expect(getByText(/your teams lives inside a Dōjō/)).toBeTruthy();
		expect(queryByText('Acme Corp')).toBeNull();
		expect(getByText('0 memberships')).toBeTruthy();
	});
});
