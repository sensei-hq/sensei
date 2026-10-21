import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
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
		const { getByText } = render(RelayDetail, { send: vi.fn() });
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
		expect(getByText(/Phase 3 of 5/)).toBeTruthy();
		expect(getByText('Run the staging migration?')).toBeTruthy();
	});

	it('renders nothing when no run is selected', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.select(null);
		const { queryByText } = render(RelayDetail, { send: vi.fn() });
		expect(queryByText('refactor refresh-token rotation')).toBeNull();
	});

	// #305: an answer in the inbox reaches the run's dōjō — it was a local echo.
	it('sends a Deny to the run’s dōjō as a deny verdict, then shows it answered', async () => {
		const [run] = relayInboxMock();
		const ask = { id: 'g-deny', action: 'approve' as const, blocking: true, prompt: 'Drop the table?', options: ['Approve', 'Deny'], createdAt: new Date().toISOString() };
		relayInboxState.load([{ ...run, asks: [ask], needs: 1 }]);
		relayInboxState.select(run.id);
		const send = vi.fn().mockResolvedValue(undefined);
		const { getByText, findByText } = render(RelayDetail, { send });
		await fireEvent.click(getByText('Deny'));
		await fireEvent.click(getByText('Send answer'));
		expect(send).toHaveBeenCalledWith('organization/acme', 'g-deny', { verdict: 'deny' });
		expect(await findByText(/· Deny/)).toBeTruthy();
	});

	it('a reply that does not land keeps the ask open and says why', async () => {
		const [run] = relayInboxMock();
		const ask = { id: 'g-fail', action: 'approve' as const, blocking: true, prompt: 'Drop the table?', options: ['Approve', 'Deny'], createdAt: new Date().toISOString() };
		relayInboxState.load([{ ...run, asks: [ask], needs: 1 }]);
		relayInboxState.select(run.id);
		const { getByText, findByText } = render(RelayDetail, { send: vi.fn().mockRejectedValue(new Error('relay unavailable')) });
		await fireEvent.click(getByText('Approve'));
		await fireEvent.click(getByText('Send answer'));
		expect(await findByText('relay unavailable')).toBeTruthy();
		expect(getByText('Send answer')).toBeTruthy();
	});
});
