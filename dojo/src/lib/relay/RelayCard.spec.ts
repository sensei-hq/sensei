import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RelayCard from './RelayCard.svelte';
import type { RelaySession } from './types';

// Component render test: a RelaySession in → the mockup card out. Locks the copy +
// the pip/why wiring; the rank/filter logic is tested in relay-inbox-state.spec.
function session(over: Partial<RelaySession> = {}): RelaySession {
	return {
		id: 'r1',
		project: 'lumen-auth',
		title: 'refactor refresh-token rotation',
		goal: null,
		status: 'running',
		done: 5,
		total: 12,
		phase: 'Implement',
		lastEventAt: null,
		needs: 1,
		attention: null,
		plan: {
			phases: [
				{ id: 'p1', title: 'A', state: 'done', tasks: [] },
				{ id: 'p2', title: 'B', state: 'active', tasks: [] },
				{ id: 'p3', title: 'C', state: 'pending', tasks: [] }
			]
		},
		asks: [],
		...over
	};
}

afterEach(cleanup);

describe('RelayCard', () => {
	it('renders repo (distinct from title), title, why-line, and done/total', () => {
		const { getByText } = render(RelayCard, { session: session() });
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
		expect(getByText('1 needs you')).toBeTruthy();
		expect(getByText('5/12')).toBeTruthy();
	});

	it('renders one plan pip per phase', () => {
		const { container } = render(RelayCard, { session: session() });
		expect(container.querySelector('[title="plan progress"]')?.children.length).toBe(3);
	});

	it('shows the attention why-line when nothing needs you', () => {
		const { getByText } = render(RelayCard, {
			session: session({ needs: 0, status: 'stalled', attention: 'stalled' })
		});
		expect(getByText('no heartbeat')).toBeTruthy();
	});

	it('marks the wrapper selected + opens on click', () => {
		const onopen = vi.fn();
		const { getByRole, container } = render(RelayCard, { session: session(), selected: true, onopen });
		expect(container.querySelector('div.border-b')?.classList.contains('bg-paper-mute')).toBe(true);
		getByRole('button').click();
		expect(onopen).toHaveBeenCalledWith('r1');
	});
});
