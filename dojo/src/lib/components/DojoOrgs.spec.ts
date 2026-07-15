import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DojoOrgs from './DojoOrgs.svelte';
import { orgs } from '$lib/dojo-data';

// user + orgs are now props (loaded from the real session server-side); the mock
// `orgs` array still serves as a fixture for these component tests.
const user = { name: 'Test User', handle: 'test@example.com', initials: 'TU' };

describe('DojoOrgs', () => {
	afterEach(cleanup);

	it('renders the signed-in user and one card per membership', () => {
		const { getByText, getAllByText } = render(DojoOrgs, { user, orgs });

		expect(getByText('Your organizations')).toBeTruthy();
		expect(getByText(user.name)).toBeTruthy();
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
		const { getAllByText } = render(DojoOrgs, { user, orgs, onEnter });

		await fireEvent.click(getAllByText('Enter')[0]);
		expect(onEnter).toHaveBeenCalledOnce();
		expect(onEnter.mock.calls[0][0].id).toBe(orgs[0].id);
	});

	it('shows an empty state (no Enter buttons) when the user belongs to no Dōjōs', () => {
		const { getByText, queryAllByText } = render(DojoOrgs, { user, orgs: [] });

		expect(getByText(/don't belong to any Dōjōs yet/)).toBeTruthy();
		expect(queryAllByText('Enter').length).toBe(0);
	});
});
