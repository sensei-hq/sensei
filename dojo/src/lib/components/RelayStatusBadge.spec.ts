import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import RelayStatusBadge from './RelayStatusBadge.svelte';
import { statusBadge } from '$lib/relay-view';

// RelayStatusBadge render tests: the shared run-status pill shows the plain-
// language label from statusBadge(status) and prepends a pulsing accent dot ONLY
// when the run is 'running'. Every other status renders the label with no dot, so
// the calm run-list surface stays unchanged apart from the new running pulse.

describe('RelayStatusBadge', () => {
	afterEach(cleanup);

	it('renders the plain-language label for a status', () => {
		const { getByText } = render(RelayStatusBadge, { status: 'done' });
		expect(getByText(statusBadge('done').label)).toBeTruthy();
	});

	it('shows the pulse dot only when running', () => {
		const { getByText, container } = render(RelayStatusBadge, { status: 'running' });
		expect(getByText(statusBadge('running').label)).toBeTruthy();
		expect(container.querySelector('.pulse-dot')).not.toBeNull();
	});

	it('shows no pulse dot for non-running statuses', () => {
		for (const status of ['paused', 'stalled', 'crashed', 'blocked', 'done', 'failed'] as const) {
			const { container } = render(RelayStatusBadge, { status });
			expect(container.querySelector('.pulse-dot')).toBeNull();
			cleanup();
		}
	});
});
