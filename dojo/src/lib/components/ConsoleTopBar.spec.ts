import { render, cleanup, fireEvent, within } from '@testing-library/svelte';
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

const globex: DojoOrg = {
	id: 'globex',
	kanji: '客',
	name: 'Globex',
	kind: 'Client',
	host: 'saas',
	url: 'github/globex',
	role: 'Maintainer',
	from: 'GitHub · repo admin',
	members: 12,
	pending: 2
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

// Chunk 5: the bare /orgs link becomes a click-to-open, keyboard-accessible
// org-switcher popover — pinned "Relay · you", one row per membership, "Your
// Dōjōs" + "＋ Create or join a Dōjō".
describe('ConsoleTopBar org-switcher popover', () => {
	afterEach(cleanup);

	it('trigger shows the current org name when a tenant is selected', () => {
		const { getByLabelText } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true
		});
		expect(getByLabelText('Switch organization').textContent).toContain('Acme');
	});

	it('trigger shows "Relay · you" when the user is solo (no membership)', () => {
		const { getByLabelText } = render(ConsoleTopBar, {
			org: undefined,
			memberships: [],
			hasMembership: false
		});
		expect(getByLabelText('Switch organization').textContent).toContain('Relay · you');
	});

	it('the popover is closed until the trigger is clicked (aria-expanded)', async () => {
		const { getByLabelText, getByRole, queryByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true
		});
		const trigger = getByLabelText('Switch organization');
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
		expect(queryByRole('menu')).toBeNull();
		await fireEvent.click(trigger);
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		expect(getByRole('menu')).toBeTruthy();
	});

	it('lists the pinned Relay · you entry and one row per membership', async () => {
		const { getByLabelText, getByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true
		});
		await fireEvent.click(getByLabelText('Switch organization'));
		const menu = getByRole('menu');
		expect(menu.textContent).toContain('Relay · you');
		expect(menu.textContent).toContain('Acme');
		expect(menu.textContent).toContain('Globex');
		expect(menu.textContent).toContain('Your Dōjōs');
		expect(menu.textContent).toContain('Create or join a Dōjō');
	});

	it('clicking a membership row invokes onSwitch with that org', async () => {
		const onSwitch = vi.fn();
		const { getByLabelText, getByText } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true,
			onSwitch
		});
		await fireEvent.click(getByLabelText('Switch organization'));
		await fireEvent.click(getByText('Globex'));
		expect(onSwitch).toHaveBeenCalledTimes(1);
		expect(onSwitch.mock.calls[0][0]).toMatchObject({ id: 'globex' });
	});

	it('clicking the pinned Relay · you entry invokes onRelayHome', async () => {
		const onRelayHome = vi.fn();
		const { getByLabelText, getByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true,
			onRelayHome
		});
		await fireEvent.click(getByLabelText('Switch organization'));
		// The pinned menu entry (not the trigger) carries the Relay home affordance.
		const menu = getByRole('menu');
		const relayEntry = within(menu).getByText('Relay · you').closest('button');
		expect(relayEntry).not.toBeNull();
		await fireEvent.click(relayEntry as HTMLElement);
		expect(onRelayHome).toHaveBeenCalledTimes(1);
	});

	it('the "Your Dōjōs" and "Create or join" entries link to /orgs', async () => {
		const { getByLabelText, getByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true
		});
		await fireEvent.click(getByLabelText('Switch organization'));
		const menu = getByRole('menu');
		const yourDojos = within(menu).getByText('Your Dōjōs').closest('a');
		const createJoin = within(menu).getByText('Create or join a Dōjō').closest('a');
		expect(yourDojos?.getAttribute('href')).toContain('/orgs');
		expect(createJoin?.getAttribute('href')).toContain('/orgs');
	});

	it('Escape closes the popover', async () => {
		const { getByLabelText, getByRole, queryByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true
		});
		const trigger = getByLabelText('Switch organization');
		await fireEvent.click(trigger);
		expect(getByRole('menu')).toBeTruthy();
		await fireEvent.keyDown(trigger, { key: 'Escape' });
		expect(queryByRole('menu')).toBeNull();
		expect(trigger.getAttribute('aria-expanded')).toBe('false');
	});

	it('switching an org closes the popover', async () => {
		const onSwitch = vi.fn();
		const { getByLabelText, getByText, queryByRole } = render(ConsoleTopBar, {
			org,
			memberships: [org, globex],
			hasMembership: true,
			onSwitch
		});
		await fireEvent.click(getByLabelText('Switch organization'));
		await fireEvent.click(getByText('Globex'));
		expect(queryByRole('menu')).toBeNull();
	});
});
