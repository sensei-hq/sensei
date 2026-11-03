// ScrMyMetrics — a developer's own metrics in the personal console.
//
// Presentational: it renders the cards it is handed. What these pin is that a
// failed read reads as a failure, never as a quiet month, and that a card shows
// its number and a line only when there is one to draw.
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import ScrMyMetrics from './ScrMyMetrics.svelte';
import type { MyMetricsView } from '$lib/my-metrics-view';

const view = (cards: MyMetricsView['cards']): MyMetricsView => ({ days: 30, cards });
const card = (over: Partial<MyMetricsView['cards'][number]> = {}) => ({
	key: 'throughput',
	label: 'Sessions',
	value: '6',
	points: [4, 2],
	empty: false,
	...over
});

describe('ScrMyMetrics', () => {
	it('shows each metric with its number over the window', () => {
		const { getByText } = render(ScrMyMetrics, {
			props: { view: view([card(), card({ key: 'ftr', label: 'First-try', value: '67%', points: [0.5, 1] })]) }
		});
		expect(getByText('Sessions')).toBeTruthy();
		expect(getByText('6')).toBeTruthy();
		expect(getByText('67%')).toBeTruthy();
		expect(getByText(/last 30 days/)).toBeTruthy();
	});

	it('draws a line from two days, and none from one (a single point has no slope)', () => {
		const { container } = render(ScrMyMetrics, {
			props: { view: view([card(), card({ key: 'ftr', label: 'First-try', value: '80%', points: [0.8] })]) }
		});
		const lines = container.querySelectorAll('polyline');
		expect(lines.length).toBe(1);
		expect(lines[0].getAttribute('points')).not.toMatch(/NaN/);
	});

	it('says a metric nothing was sent for, rather than showing a zero', () => {
		const { getByText } = render(ScrMyMetrics, {
			props: { view: view([card({ key: 'tokens_per_day', label: 'Tokens', value: '—', points: [], empty: true })]) }
		});
		expect(getByText(/Nothing sent in this window/)).toBeTruthy();
	});

	it('a failed read is an error, not an empty month', () => {
		const { getByRole, queryByText } = render(ScrMyMetrics, {
			props: { view: null, error: 'the request failed' }
		});
		expect(getByRole('alert').textContent).toMatch(/Could not load your metrics — the request failed/);
		expect(queryByText('Sessions')).toBeNull();
	});
});
