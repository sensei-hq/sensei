// A failed LOAD replaces the lists: drawn with a count of 0 it would read as
// "you have none", the answer a failure must never pass for.
import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import ScrRulePacks from './ScrRulePacks.svelte';
import ScrContributions from './ScrContributions.svelte';

afterEach(() => cleanup());

describe('a screen whose read failed', () => {
	it('rule packs says so, and draws no Adopted or Available list', () => {
		const { getByRole, queryByText } = render(ScrRulePacks, { props: { packs: [], loadError: 'the request failed' } });
		expect(getByRole('alert').textContent).toMatch(/Could not load the rule packs — the request failed/);
		expect(queryByText('Adopted')).toBeNull();
		expect(queryByText('Available')).toBeNull();
	});

	it('contributions says so, and draws neither list', () => {
		const { getByRole, queryByText } = render(ScrContributions, {
			props: { mine: [], downstream: [], loadError: 'the request failed' }
		});
		expect(getByRole('alert').textContent).toMatch(/Could not load your contributions — the request failed/);
		expect(queryByText("What you've shared")).toBeNull();
		expect(queryByText('Approved for you')).toBeNull();
	});
});
