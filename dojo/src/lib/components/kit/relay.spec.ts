import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ChatThread from './ChatThread.svelte';
import RunCardHarness from './RunCard.harness.svelte';
import GateCardHarness from './GateCard.harness.svelte';
import DecisionCardHarness from './DecisionCard.harness.svelte';
import NeedsYouBandHarness from './NeedsYouBand.harness.svelte';
import { runs, chat, me, needsYou, gates } from './fixtures';

// Render smoke tests for the relay-plane domain components (run / gate / needs /
// decision / chat). Each mounts with a fixture, asserts key content + variants,
// and — critically — that the needs-you band's per-kind action set fires the
// right callback and that resolved / empty states render.
describe('kit relay components render', () => {
	afterEach(cleanup);

	it('RunCard shows the task, session meta, status and gate chip', () => {
		const { getByText } = render(RunCardHarness, { run: runs[0] });
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
		expect(getByText(/s-2891/)).toBeTruthy();
		expect(getByText('running')).toBeTruthy();
		expect(getByText('gate waiting')).toBeTruthy();
		expect(getByText(/12 edits/)).toBeTruthy();
	});

	it('RunCard shows the waiting status for a non-running run', () => {
		const { getByText } = render(RunCardHarness, { run: runs[1] });
		expect(getByText('waiting')).toBeTruthy();
	});

	it('RunCard fires onOpen when clicked', async () => {
		const { getByText, getByTestId } = render(RunCardHarness, { run: runs[0] });
		await fireEvent.click(getByText('refactor refresh-token rotation'));
		expect(getByTestId('opens').textContent).toBe('1');
	});

	it('RunCard stacked variant renders the phone layout', () => {
		const { getByText } = render(RunCardHarness, { run: runs[0], stacked: true });
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
		// the stacked gate chip reads just "gate".
		expect(getByText('gate')).toBeTruthy();
	});

	it('GateCard shows the command, risk chip, why + session, and fires approve/deny', async () => {
		const { getByText, getByTestId } = render(GateCardHarness, { gate: undefined });
		expect(getByText(/pnpm db:migrate --env=staging/)).toBeTruthy();
		expect(getByText('guarded')).toBeTruthy();
		expect(getByText(/touches an auth-boundary schema · session s-2891/)).toBeTruthy();
		await fireEvent.click(getByText('Approve once'));
		expect(getByTestId('approves').textContent).toBe('1');
		await fireEvent.click(getByText('Deny'));
		expect(getByTestId('denies').textContent).toBe('1');
	});

	it('GateCard tints a high-risk gate in danger', () => {
		const { getByText } = render(GateCardHarness, { gate: gates[1] });
		expect(getByText('high').className).toContain('text-danger');
	});

	it('DecisionCard shows the title, context, options and forwards the choice', async () => {
		const { getByText, getByTestId } = render(DecisionCardHarness, {});
		expect(getByText('adopt ‘verify webhook signature’ as a client guard')).toBeTruthy();
		expect(getByText(/4 sessions · dereferenced · confidence 0.91/)).toBeTruthy();
		await fireEvent.click(getByText('adopt to Client rung'));
		expect(getByTestId('chosen').textContent).toBe('adopt to Client rung');
	});

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
