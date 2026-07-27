import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import PlanPips from './PlanPips.svelte';
import type { KitPlan } from './types';

// PlanPips renders one pip per phase (a parallel phase splits into two) and an
// optional done/total caption. The tone/roll-up logic lives in plan.ts (tested
// there); this locks the pip count and the caption.
const PLAN: KitPlan = {
	phases: [
		// parallel: two tasks with no deps → two pips
		{
			id: 'a',
			title: 'A',
			tasks: [
				{ id: 't1', title: 't1', state: 'done' },
				{ id: 't2', title: 't2', state: 'done' }
			]
		},
		// sequential: one pip
		{ id: 'b', title: 'B', tasks: [{ id: 't3', title: 't3', state: 'active', deps: ['t1'] }] }
	]
};

afterEach(cleanup);

describe('PlanPips', () => {
	it('renders a caption of done/total tasks', () => {
		const { getByText } = render(PlanPips, { plan: PLAN });
		expect(getByText('2/3 tasks')).toBeTruthy(); // 2 done of 3
	});

	it('splits a parallel phase into two pips and a sequential into one', () => {
		const { container } = render(PlanPips, { plan: PLAN });
		// pips are the round-full spans inside the phase groups
		const pips = container.querySelectorAll('span.rounded-full');
		expect(pips.length).toBe(3); // 2 (parallel) + 1 (sequential)
	});

	it('renders nothing for an empty plan', () => {
		const { container } = render(PlanPips, { plan: [] });
		expect(container.querySelector('span')).toBeNull();
	});

	it('hides the caption when caption=false', () => {
		const { queryByText } = render(PlanPips, { plan: PLAN, caption: false });
		expect(queryByText('2/3 tasks')).toBeNull();
	});
});
