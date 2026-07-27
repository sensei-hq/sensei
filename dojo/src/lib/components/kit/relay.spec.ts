import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ChatThread from './ChatThread.svelte';
import NeedsYouBandHarness from './NeedsYouBand.harness.svelte';
import { chat, me, needsYou } from './fixtures';

// Render smoke tests for the surviving relay-plane kit components (needs-you band
// + chat thread). Each mounts with a fixture, asserts key content, and — critically
// — that the needs-you band's per-kind action set fires the right callback and that
// resolved / empty states render.
describe('kit relay components render', () => {
	afterEach(cleanup);

	it('ChatThread renders sensei + viewer turns with their bylines', () => {
		const { getByText, getAllByText } = render(ChatThread, { thread: chat, me });
		expect(getByText(/Noticed the refresh-token rotation touches the logger/)).toBeTruthy();
		expect(getByText('Approving now.')).toBeTruthy();
		// sensei speaks (byline) at least once.
		expect(getAllByText(/sensei ·/).length).toBeGreaterThan(0);
	});

	it('NeedsYouBand shows the header count and a row per item', () => {
		const { getByText } = render(NeedsYouBandHarness, {});
		expect(getByText('Needs you')).toBeTruthy();
		expect(getByText('run migration against staging db')).toBeTruthy();
		expect(getByText('retry policy clashes with idempotency rule')).toBeTruthy();
	});

	it('NeedsYouBand fires the per-kind action: gate → Approve', async () => {
		const { getAllByText, getByTestId } = render(NeedsYouBandHarness, {});
		// n1 is the gate row → its primary action is "Approve".
		await fireEvent.click(getAllByText('Approve')[0]);
		expect(getByTestId('last-need').textContent).toBe('n1');
		expect(getByTestId('last-action').textContent).toBe('approve');
	});

	it('NeedsYouBand fires the per-kind action: conflict → Settle', async () => {
		const { getByText, getByTestId } = render(NeedsYouBandHarness, {});
		await fireEvent.click(getByText('Settle'));
		expect(getByTestId('last-need').textContent).toBe('n2');
		expect(getByTestId('last-action').textContent).toBe('settle');
	});

	it('NeedsYouBand fires the per-kind action: decision → Decide', async () => {
		const { getByText, getByTestId } = render(NeedsYouBandHarness, {});
		await fireEvent.click(getByText('Decide'));
		expect(getByTestId('last-need').textContent).toBe('n3');
		expect(getByTestId('last-action').textContent).toBe('decide');
	});

	it('NeedsYouBand review row → Approve fires with the review item', async () => {
		// isolate the review row (n4) so its "Approve" is unambiguous.
		const { getByText, getByTestId } = render(NeedsYouBandHarness, {
			items: needsYou.filter((n) => n.kind === 'review')
		});
		await fireEvent.click(getByText('Approve'));
		expect(getByTestId('last-need').textContent).toBe('n4');
		expect(getByTestId('last-action').textContent).toBe('approve');
	});

	it('NeedsYouBand shows a resolved marker instead of actions for a resolved item', () => {
		const { getByText, queryByText } = render(NeedsYouBandHarness, {
			items: needsYou.filter((n) => n.kind === 'gate'),
			resolved: { n1: 'approved' }
		});
		expect(getByText('approved')).toBeTruthy();
		// the action buttons are gone once resolved.
		expect(queryByText('Approve')).toBeNull();
		expect(queryByText('Deny')).toBeNull();
	});

	it('NeedsYouBand shows the empty state when nothing is waiting', () => {
		const { getByText } = render(NeedsYouBandHarness, { items: [] });
		expect(getByText('Nothing needs you.')).toBeTruthy();
	});

	it('NeedsYouBand fires onOpen when a row body is clicked', async () => {
		const { getByText, getByTestId } = render(NeedsYouBandHarness, {});
		await fireEvent.click(getByText('run migration against staging db'));
		expect(getByTestId('opened').textContent).toBe('n1');
	});
});
