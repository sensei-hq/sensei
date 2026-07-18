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
			tenantKey: 't',
			open: true,
			onClose
		});
		await fireEvent.click(getByLabelText('Close navigation'));
		expect(onClose).toHaveBeenCalledTimes(1);
	});

	it('renders no backdrop when closed', () => {
		const { queryByLabelText } = render(ConsoleNav, {
			active: 'overview',
			tenantKey: 't',
			open: false
		});
		expect(queryByLabelText('Close navigation')).toBeNull();
	});

	it('closes the drawer when a wired destination is tapped', async () => {
		const onClose = vi.fn();
		const { getByText } = render(ConsoleNav, {
			active: 'overview',
			tenantKey: 't',
			open: true,
			onClose
		});
		await fireEvent.click(getByText('Triage'));
		expect(onClose).toHaveBeenCalled();
	});
});
