import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ConsoleNav from './ConsoleNav.svelte';

// On phone widths the nav is an off-canvas drawer: a backdrop dims the page and
// closes it, and navigating (tapping a destination) closes it too. At md:+ it is a
// static sidebar with no backdrop, so `open` is irrelevant there.
describe('ConsoleNav drawer behavior', () => {
	afterEach(cleanup);

	it('renders a backdrop that closes the drawer when open', async () => {
		const onClose = vi.fn();
		const { getByLabelText } = render(ConsoleNav, {
			active: 'overview',
			open: true,
			onClose
		});
		await fireEvent.click(getByLabelText('Close navigation'));
		expect(onClose).toHaveBeenCalledTimes(1);
	});

	it('renders no backdrop when closed', () => {
		const { queryByLabelText } = render(ConsoleNav, {
			active: 'overview',
			open: false
		});
		expect(queryByLabelText('Close navigation')).toBeNull();
	});

	it('closes the drawer when a wired destination is tapped', async () => {
		const onClose = vi.fn();
		const { getByText } = render(ConsoleNav, {
			active: 'overview',
			open: true,
			onClose
		});
		await fireEvent.click(getByText('Triage'));
		expect(onClose).toHaveBeenCalled();
	});
});

// Chunk 5 IA reframe: the personal zone (Relay · you + Me) renders on top; the
// management groups (Govern · Org · Clients · Trust) render below a divider,
// de-emphasized. Every currently-wired route stays reachable.
describe('ConsoleNav IA structure', () => {
	afterEach(cleanup);

	it('renders the personal-zone group headers on top', () => {
		const { getByText } = render(ConsoleNav, { active: 'relay' });
		expect(getByText('Relay · you')).toBeTruthy();
		expect(getByText('Me')).toBeTruthy();
	});

	it('renders the management group headers', () => {
		const { getByText } = render(ConsoleNav, { active: 'overview' });
		expect(getByText('Govern')).toBeTruthy();
		expect(getByText('Org')).toBeTruthy();
		expect(getByText('Clients')).toBeTruthy();
		expect(getByText('Trust')).toBeTruthy();
	});

	it('orders the personal zone before the first management group in the DOM', () => {
		const { getByText } = render(ConsoleNav, { active: 'overview' });
		const me = getByText('Me');
		const govern = getByText('Govern');
		// Node.DOCUMENT_POSITION_FOLLOWING (4) means govern comes after me.
		expect(me.compareDocumentPosition(govern) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
	});

	it('renders a divider separating the personal zone from the management groups', () => {
		const { getByTestId } = render(ConsoleNav, { active: 'overview' });
		expect(getByTestId('nav-manage-divider')).toBeTruthy();
	});

	it('keeps every wired console destination reachable as a link', () => {
		const { getByText } = render(ConsoleNav, { active: 'overview' });
		const wiredLabels = [
			'Relay',
			'My teams',
			'My contributions',
			'For me',
			'Overview',
			'Triage',
			'Library',
			'Effective constitution',
			'Members & roles',
			'Identities',
			'Policies',
			'Engagements',
			'Incidents',
			'Health',
			'Audit trail'
		];
		for (const label of wiredLabels) {
			const el = getByText(label);
			// Wired items render as <a href>; only soon-placeholders are plain divs.
			expect(el.closest('a')).not.toBeNull();
		}
	});

	it('marks the active destination with aria-current', () => {
		const { getByText } = render(ConsoleNav, { active: 'library' });
		expect(getByText('Library').closest('a')?.getAttribute('aria-current')).toBe('page');
		expect(getByText('Triage').closest('a')?.getAttribute('aria-current')).toBeNull();
	});

	it('keeps the version-stamp footer', () => {
		const { getByTestId } = render(ConsoleNav, { active: 'overview' });
		expect(getByTestId('dojo-version')).toBeTruthy();
	});
});
