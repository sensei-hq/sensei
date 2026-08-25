import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TopBar from './TopBar.svelte';
import NavPane from './NavPane.svelte';
import TabBar from './TabBar.svelte';
import OrgSwitcher from './OrgSwitcher.svelte';
import AppShellHarness from './AppShell.harness.svelte';
import { me, dojos, org, nav, tabs } from './fixtures';

// The chrome mounts in both contexts ("you" personal + "org"), stays mobile-first
// (a hamburger under md, an off-canvas NavPane drawer), and wires the switch /
// nav / needs callbacks. Specs drive those with fixtures from the mock data.
describe('kit chrome — TopBar', () => {
	afterEach(cleanup);

	it('renders the brand + a needs-you badge and fires onneeds', async () => {
		const onneeds = vi.fn();
		const { getByText, getByTitle } = render(TopBar, {
			props: { context: 'you', dojos, me, needsCount: 6, onneeds }
		});
		expect(getByText('Dōjō')).toBeTruthy();
		await fireEvent.click(getByTitle('6 need you'));
		expect(onneeds).toHaveBeenCalledTimes(1);
	});

	it('carries the org top-rule + route chip in org context', () => {
		const { getByText, container } = render(TopBar, {
			props: { context: 'org', org, dojos, me }
		});
		expect(getByText('sensei-hq.com/acme')).toBeTruthy();
		// the top-rule is an accent border on the bar root.
		expect((container.firstElementChild as HTMLElement).className).toContain('border-t-accent');
	});

	it('exposes an md:hidden hamburger that fires onmenu', async () => {
		const onmenu = vi.fn();
		const { getByLabelText } = render(TopBar, { props: { context: 'you', me, onmenu } });
		const burger = getByLabelText('Open navigation');
		expect(burger.className).toContain('md:hidden');
		await fireEvent.click(burger);
		expect(onmenu).toHaveBeenCalled();
	});
});

describe('kit chrome — NavPane', () => {
	afterEach(cleanup);

	it('renders grouped items + marks the active one with aria-current', () => {
		const { getByText } = render(NavPane, { groups: nav, active: 'projects' });
		expect(getByText('Relay · you')).toBeTruthy();
		expect(getByText('Me')).toBeTruthy();
		expect(getByText('Projects').closest('button')?.getAttribute('aria-current')).toBe('page');
		expect(getByText('Today').closest('button')?.getAttribute('aria-current')).toBeNull();
	});

	it('is an off-canvas drawer under md — a backdrop closes it when open', async () => {
		const onclose = vi.fn();
		const { getByLabelText } = render(NavPane, { groups: nav, active: 'today', open: true, onclose });
		await fireEvent.click(getByLabelText('Close navigation'));
		expect(onclose).toHaveBeenCalledTimes(1);
	});

	it('renders no backdrop when closed and the aside is static at md:+', () => {
		const { queryByLabelText, container } = render(NavPane, { groups: nav, active: 'today' });
		expect(queryByLabelText('Close navigation')).toBeNull();
		expect(container.querySelector('aside')?.className).toContain('md:static');
	});

	it('fires onnav and onclose when a destination is chosen', async () => {
		const onnav = vi.fn();
		const onclose = vi.fn();
		const { getByText } = render(NavPane, {
			groups: nav,
			active: 'today',
			open: true,
			onnav,
			onclose
		});
		await fireEvent.click(getByText('Projects'));
		expect(onnav).toHaveBeenCalledWith('projects');
		expect(onclose).toHaveBeenCalled();
	});
});

describe('kit chrome — TabBar', () => {
	afterEach(cleanup);

	it('renders one tab per item and fires onnav', async () => {
		const onnav = vi.fn();
		const { getByText } = render(TabBar, { tabs, active: 'today', onnav });
		expect(getByText('Today')).toBeTruthy();
		await fireEvent.click(getByText('Projects'));
		expect(onnav).toHaveBeenCalledWith('projects');
	});
});

describe('kit chrome — OrgSwitcher', () => {
	afterEach(cleanup);

	it('opens the popover and lists memberships; picking one fires onpick', async () => {
		const onpick = vi.fn();
		const { getByText, getAllByText } = render(OrgSwitcher, {
			props: { context: 'you', dojos, onpick }
		});
		// closed: the trigger shows the personal label.
		expect(getAllByText('Your work').length).toBeGreaterThan(0);
		await fireEvent.click(getAllByText('Your work')[0].closest('button')!);
		// open: the membership list is visible.
		expect(getByText('Acme Corp')).toBeTruthy();
		await fireEvent.click(getByText('Acme Corp').closest('button')!);
		expect(onpick).toHaveBeenCalledWith('acme');
	});
});

describe('kit chrome — the one shell composes', () => {
	afterEach(cleanup);

	it('AppShell wraps TopBar + NavPane around the main content', () => {
		const { getByText, container } = render(AppShellHarness, {
			props: { context: 'you', dojos, me, nav, active: 'today' }
		});
		expect(getByText('Dōjō')).toBeTruthy();
		expect(getByText('the main column content')).toBeTruthy();
		expect(container.querySelector('aside#kit-nav')).toBeTruthy();
	});

	it('AppShell carries the phone tab bar, hidden from md up', () => {
		// The phone chrome lives in this same shell rather than a second one, so
		// `md:hidden` — not a separate mount — is what keeps the tabs off desktop.
		// Asserted structurally: TopBar's hamburger is also `md:hidden`, and "Today"
		// appears in the NavPane groups too, so neither is a unique handle.
		const { container } = render(AppShellHarness, {
			props: { context: 'org', org, me, nav, tabs, active: 'today' }
		});
		const shell = container.firstElementChild!;
		expect(shell.children.length).toBe(3); // TopBar · nav+main row · tab bar
		const tabWrap = shell.lastElementChild as HTMLElement;
		expect(tabWrap.className).toContain('md:hidden');
		expect(tabWrap.querySelectorAll('button').length).toBe(tabs.length);
		expect(container.querySelector('aside#kit-nav')).toBeTruthy();
	});

	it('AppShell renders its main content exactly once', () => {
		// The regression guard for the duplicate-render this shell replaced: the
		// (app) layout used to mount a desktop shell AND a phone shell, each
		// rendering `children()`, so every screen existed twice in the DOM.
		const { container } = render(AppShellHarness, {
			props: { context: 'org', org, me, nav, tabs, active: 'today' }
		});
		expect(container.querySelectorAll('main').length).toBe(1);
	});

	it('AppShell omits the tab bar entirely when no tabs are supplied', () => {
		const { container } = render(AppShellHarness, {
			props: { context: 'you', dojos, me, nav, active: 'today' }
		});
		const shell = container.firstElementChild!;
		expect(shell.children.length).toBe(2); // TopBar · nav+main row — no tab bar
	});
});
