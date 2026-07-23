import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoDeveloperDownstream from './DojoDeveloperDownstream.svelte';
import { DOWNSTREAM } from '$lib/developer-data';

// Render tests for "For me" — the downstream (approved-teachings) surface. Proves
// each teaching renders from the catalog (title, origin, new/adopted chip, the
// mute/pin controls) for a member, and that a solo (membership-less) contributor
// gets the honest empty state ("no teachings yet") instead of fabricated rows
// (DJ1 — a personal surface, no join-gate).

describe('DojoDeveloperDownstream', () => {
	afterEach(() => cleanup());

	it('renders the header and a row per teaching for a member', () => {
		const { getByText } = render(DojoDeveloperDownstream, { hasMembership: true });
		expect(getByText('Approved for you')).toBeTruthy();
		for (const it of DOWNSTREAM) {
			expect(getByText(it.title)).toBeTruthy();
		}
	});

	it('marks adopted vs new and offers mute / pin per row', () => {
		const { getByText, getAllByText } = render(DojoDeveloperDownstream, { hasMembership: true });
		// 1 adopted, 2 new.
		expect(getByText('✓ adopted')).toBeTruthy();
		expect(getAllByText('new').length).toBe(2);
		// Every row has a mute + pin control.
		expect(getAllByText('mute').length).toBe(DOWNSTREAM.length);
		expect(getAllByText('pin').length).toBe(DOWNSTREAM.length);
	});

	it('shows the honest empty state for a membership-less contributor', () => {
		const { getByText, queryByText } = render(DojoDeveloperDownstream, { hasMembership: false });
		expect(getByText('no teachings yet')).toBeTruthy();
		expect(getByText(/join or create a Dōjō from the switcher/)).toBeTruthy();
		expect(queryByText(DOWNSTREAM[0].title)).toBeNull();
	});
});
