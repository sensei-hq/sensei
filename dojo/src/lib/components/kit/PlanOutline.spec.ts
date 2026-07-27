import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import PlanOutline from './PlanOutline.svelte';
import type { KitPlan } from './types';

// PlanOutline renders phases → tasks with a done/total header, per-task meta
// (agent · model · spec_ref · summary), and a gate chip. The plan normalizers +
// tones are tested elsewhere; this locks the outline's copy.
const PLAN: KitPlan = {
	phases: [
		{
			id: 'p1',
			title: 'Design',
			mode: 'sequential',
			tasks: [
				{ id: 't1', title: 'sketch', state: 'done', agent: 'coder', model: 'opus', deps: [] },
				{
					id: 't2',
					title: 'gate step',
					state: 'needs_review',
					is_gate: true,
					gate_severity: 'blocking',
					deps: []
				}
			]
		}
	]
};

afterEach(cleanup);

describe('PlanOutline', () => {
	it('renders the phase header with a done/total count', () => {
		const { getByText } = render(PlanOutline, { plan: PLAN });
		expect(getByText('Design')).toBeTruthy();
		expect(getByText('1/2')).toBeTruthy(); // one done of two tasks
	});

	it('renders task titles, the agent·model meta, and a blocking gate chip', () => {
		const { getByText } = render(PlanOutline, { plan: PLAN });
		expect(getByText('sketch')).toBeTruthy();
		expect(getByText(/coder · opus/)).toBeTruthy();
		expect(getByText('gate · blocking')).toBeTruthy();
	});
});
