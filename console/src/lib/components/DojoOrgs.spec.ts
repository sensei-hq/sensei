import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DojoOrgs from './DojoOrgs.svelte';
import { orgs } from '$lib/dojo-data';

describe('DojoOrgs', () => {
	afterEach(cleanup);

	it('renders the org picker heading and one card per membership', () => {
		const { getByText, getAllByText } = render(DojoOrgs);

		expect(getByText('Your organizations')).toBeTruthy();
		// Every org name renders.
		for (const org of orgs) {
			expect(getByText(org.name)).toBeTruthy();
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
