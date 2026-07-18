import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import RelayBlockedHome from './RelayBlockedHome.svelte';
import type { RelayGate } from '$lib/relay-data';

// RelayBlockedHome (P4.6 "blocked on you" home) render tests: the away-from-keyboard
// landing that aggregates the pending gates across every run. It shows them
// urgency-ordered (blocking before advisory, then oldest-waiting first — via
// relay-view.orderGatesByUrgency), each row deep-linking to its run's gate card
// (gateHref), and a calm empty state when nothing's blocked. Presentational only —
// it takes `gates` as a prop, so it renders without a live backend or the page load.

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

function gate(overrides: Partial<RelayGate> = {}): RelayGate {
	return {
		id: 'g1',
		seq: 1,
		run_id: 'run-1',
		run_title: 'Round-trip',
		segment_id: null,
		kind: 'approval',
		payload: { prompt: 'Run the prod migration?' },
		created_at: '2026-07-18T12:00:00.000Z',
		...overrides
	};
}

describe('RelayBlockedHome', () => {
	afterEach(() => cleanup());

	it('shows the empty state (and no gate rows) when nothing is blocked', () => {
		const { getByText, container } = render(RelayBlockedHome, { gates: [] });
		expect(getByText(/Nothing's waiting on you/)).toBeTruthy();
		// No deep-links in the empty state.
		expect(container.querySelectorAll('a[href^="/console/relay/"]').length).toBe(0);
	});

	it('renders one row per gate with its ask and how long it has waited', () => {
		const { getByText } = render(RelayBlockedHome, {
			gates: [gate({ id: 'a', payload: { prompt: 'Run the prod migration?' } })]
		});
		expect(getByText('Run the prod migration?')).toBeTruthy();
	});

	it('falls back to a friendly ask when the payload carries no prompt', () => {
		const { getByText } = render(RelayBlockedHome, { gates: [gate({ payload: {} })] });
		expect(getByText('The run needs a decision')).toBeTruthy();
	});

	it('orders gates by urgency — blocking-oldest first, advisory last', () => {
		const gates = [
			gate({ id: 'adv', kind: 'chat', run_id: 'run-adv', payload: { prompt: 'ADV' } }),
			gate({
				id: 'blk-new',
				kind: 'approval',
				run_id: 'run-blk-new',
				payload: { prompt: 'BLK-NEW' },
				created_at: '2026-07-18T12:00:00.000Z'
			}),
			gate({
				id: 'blk-old',
				kind: 'approval',
				run_id: 'run-blk-old',
				payload: { prompt: 'BLK-OLD' },
				created_at: '2026-07-18T08:00:00.000Z'
			})
		];
		const { container } = render(RelayBlockedHome, { gates });
		// Deep-links appear in urgency order: blocking-oldest, blocking-newer, advisory.
		const hrefs = [...container.querySelectorAll('a[href^="/console/relay/"]')].map((a) =>
			a.getAttribute('href')
		);
		expect(hrefs).toEqual([
			'/console/relay/run-blk-old',
			'/console/relay/run-blk-new',
			'/console/relay/run-adv'
		]);
	});

	it('deep-links each row at its run and drops the link for the all-zeros uuid', () => {
		const { container } = render(RelayBlockedHome, {
			gates: [
				gate({ id: 'linked', run_id: 'run-42', payload: { prompt: 'Linked ask' } }),
				gate({ id: 'orphan', run_id: ZERO_UUID, run_title: null, payload: { prompt: 'Orphan ask' } })
			]
		});
		const hrefs = [...container.querySelectorAll('a[href^="/console/relay/"]')].map((a) =>
			a.getAttribute('href')
		);
		// Exactly one deep-link — the orphan (zero-uuid) run has none.
		expect(hrefs).toEqual(['/console/relay/run-42']);
	});
});
