import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ConsoleTopBar from './ConsoleTopBar.svelte';
import type { DojoOrg } from '$lib/dojo-data';

// A fully-typed org fixture so the top bar renders its chrome (brand · switcher ·
// search · members · avatar) without a live backend.
const org: DojoOrg = {
	id: 'o1',
	kanji: '道',
	name: 'Acme',
	kind: 'Personal',
	host: 'self',
	url: 'https://acme.test',
	role: 'admin',
	from: 'today',
	members: 4,
	pending: 0
};

// The top bar carries the mobile nav trigger (hamburger) that opens the drawer on
// phone widths — the desktop console never needs it (the sidebar is always visible
// at md:+), so it is md:hidden but always present in the DOM for the layout to wire.
describe('ConsoleTopBar responsive chrome', () => {
	afterEach(cleanup);

	it('exposes a mobile menu button that invokes onMenu', async () => {
		const onMenu = vi.fn();
		const { getByLabelText } = render(ConsoleTopBar, { org, onMenu });
		await fireEvent.click(getByLabelText('Open navigation'));
		expect(onMenu).toHaveBeenCalledTimes(1);
	});

	it('renders the menu button even without an onMenu handler (no throw)', () => {
		const { getByLabelText } = render(ConsoleTopBar, { org });
		expect(getByLabelText('Open navigation')).toBeTruthy();
	});

	it('reflects the drawer open state onto the trigger via aria-expanded', () => {
		const closed = render(ConsoleTopBar, { org, navExpanded: false });
		expect(closed.getByLabelText('Open navigation').getAttribute('aria-expanded')).toBe('false');
		cleanup();
		const open = render(ConsoleTopBar, { org, navExpanded: true });
		expect(open.getByLabelText('Open navigation').getAttribute('aria-expanded')).toBe('true');
	});
});
