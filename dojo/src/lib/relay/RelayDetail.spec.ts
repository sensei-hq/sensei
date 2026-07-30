import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import RelayDetail from './RelayDetail.svelte';
import { relayInboxState } from './relay-inbox-state.svelte';
import { relayInboxMock } from './relay-inbox.mock';

// Render test: the selected RelaySession (from state) → the mockup RunDetail. Locks
// the header identity, the phase-of-phases line, and the asks tab. RelayDetail reads
// the module singleton, so we seed it (load + select) before rendering.
afterEach(cleanup);

describe('RelayDetail', () => {
	it('renders the selected run header, phase line, and its asks', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.select('run-lumen');
		const { getByText } = render(RelayDetail);
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
		expect(getByText(/Phase 3 of 5/)).toBeTruthy();
		expect(getByText('Run the staging migration?')).toBeTruthy();
	});

	it('renders nothing when no run is selected', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.select(null);
		const { queryByText } = render(RelayDetail);
		expect(queryByText('refactor refresh-token rotation')).toBeNull();
	});
});
