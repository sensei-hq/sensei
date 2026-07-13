import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DojoOrgs from './DojoOrgs.svelte';
import { orgs } from '$lib/dojo-data';

describe('DojoOrgs', () => {
	afterEach(cleanup);

	it('renders the org picker heading and one card per membership', () => {
		const { getByText, getAllByText } = render(DojoOrgs);

		expect(getByText('Your organizations')).toBeTruthy();
		// Every org name renders. Use getAllByText (>=1): an org named after its own
		// kind (e.g. "Personal") also matches the kind chip, so getByText's
		// exactly-one contract would false-fail on that legitimate collision.
		for (const org of orgs) {
			expect(getAllByText(org.name).length).toBeGreaterThan(0);
		}
		// One Enter button per org.
		expect(getAllByText('Enter').length).toBe(orgs.length);
	});

	it('invokes onEnter with the org when its Enter button is clicked', async () => {
		const onEnter = vi.fn();
		const { getAllByText } = render(DojoOrgs, { onEnter });

		await fireEvent.click(getAllByText('Enter')[0]);
		expect(onEnter).toHaveBeenCalledOnce();
		expect(onEnter.mock.calls[0][0].id).toBe(orgs[0].id);
	});
});
