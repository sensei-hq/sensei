// A workspace's delivery facts as the dōjō shows them (tracker-delivery-metrics D5;
// #352). What must never happen: a null figure shown as 0 or as its bare code, a
// real 0 shown as a reason, sprints re-sorted away from the daemon's order.
import { describe, it, expect } from 'vitest';
import { toDeliveryView } from './workspace-delivery-view';
import type { WorkspaceDeliveryWire } from './workspace-delivery-data';

const sdlc = (over: Record<string, unknown> = {}) => ({
	commitment_reliability: 80,
	total_completion: 90,
	scope_change: 0,
	carry_over_rate: 20,
	unplanned_work_ratio: 10,
	velocity: 0,
	points_commitment_reliability: null,
	facts: null,
	reasons: { points_commitment_reliability: 'history_pending' },
	...over
});

const wire = (): WorkspaceDeliveryWire => ({
	boards: [
		{
			board_id: 'b1',
			board_key: 'ab'.repeat(32),
			kind: 'jira',
			name: 'Mobile Scrum',
			predictability: null,
			predictability_reason: 'too_few_sprints',
			predictability_words: 'Too few sprints',
			history_at: '2026-11-01T10:00:00Z',
			pending_tickets: 0,
			reported_at: '2026-11-04T10:00:00Z',
			sprints: [
				{
					external_id: '102',
					seq: 102,
					name: 'Sprint 2',
					state: 'active',
					start_at: '2026-10-15T00:00:00Z',
					end_at: '2026-10-28T00:00:00Z',
					complete_at: null,
					sdlc: sdlc(),
					reason_words: { history_pending: 'Ticket history not read yet' }
				},
				{
					external_id: '103',
					seq: 103,
					name: 'Sprint 3',
					state: 'future',
					start_at: null,
					end_at: null,
					complete_at: null,
					sdlc: null,
					reason_words: null
				},
				{
					external_id: '101',
					seq: 101,
					name: 'Sprint 1',
					state: 'closed',
					start_at: '2026-10-01T00:00:00Z',
					end_at: '2026-10-14T00:00:00Z',
					complete_at: '2026-10-12T00:00:00Z',
					sdlc: sdlc({ reasons: {}, points_commitment_reliability: 75 }),
					reason_words: null
				}
			]
		}
	]
});

describe('toDeliveryView', () => {
	it("keeps the board's name, its predictability's words, and its sprints in the order read", () => {
		const [b] = toDeliveryView(wire()).boards;
		expect(b.name).toBe('Mobile Scrum');
		expect(b.predictability).toMatchObject({ text: 'Too few sprints', reason: true });
		expect(b.sprints.map((s) => s.id)).toEqual(['102', '103', '101']);
	});

	it('shows each sprint’s seven figures: a real 0 as 0, a null as its reason’s words', () => {
		const s = toDeliveryView(wire()).boards[0].sprints[0];
		expect(s.figures.map((f) => f.text)).toEqual(['80%', '90%', '0%', '20%', '10%', '0 pts', 'Ticket history not read yet']);
		expect(s.figures.at(-1)!.reason).toBe(true);
	});

	it('a sprint with no figures shows “—”, never 0', () => {
		const s = toDeliveryView(wire()).boards[0].sprints[1];
		expect(s.figures).toHaveLength(7);
		expect(s.figures.every((f) => f.text === '—')).toBe(true);
	});

	it('dates run from the start to the close, else the planned end; none says so', () => {
		const [active, future, closed] = toDeliveryView(wire()).boards[0].sprints;
		expect(active.dates).toBe('Oct 15 – Oct 28');
		expect(closed.dates).toBe('Oct 1 – Oct 12');
		expect(future.dates).toBe('No dates');
	});

	it('a predictability with a value shows it in points', () => {
		const w = wire();
		w.boards[0].predictability = 12.5;
		w.boards[0].predictability_reason = null;
		w.boards[0].predictability_words = null;
		expect(toDeliveryView(w).boards[0].predictability).toMatchObject({ text: '12.5 pts', reason: false });
	});
});
