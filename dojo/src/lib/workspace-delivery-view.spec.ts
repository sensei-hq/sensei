// A workspace's delivery facts as the dōjō shows them (tracker-delivery-metrics D5;
// #352). What must never happen: a null figure shown as 0 or as its bare code, a
// real 0 shown as a reason, sprints re-sorted away from the daemon's order.
import { describe, it, expect } from 'vitest';
import { teamView, toDeliveryView, TICKET_TEAM } from './workspace-delivery-view';
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

/** Nobody reported effort on the item. */
const noTeam = { team: null, team_words: null, contributors: 0, last_report: null };

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
			...noTeam,
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
					reason_words: { history_pending: 'Ticket history not read yet' },
					...noTeam
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
					reason_words: null,
					...noTeam
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
					reason_words: null,
					...noTeam
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

describe('teamView', () => {
	const ticketTeam = {
		team: {
			values: {
				worked_minutes: 90,
				prompts: 6,
				ai_token_cost: 3,
				cost_per_prompt_cycle: null,
				total_defects: 0,
				build_failure_rate: 50,
				effort_cost: 120
			},
			reasons: { cost_per_prompt_cycle: 'partial_cost' },
			partial: { ai_token_cost: 1 }
		},
		team_words: { partial_cost: 'Cost is incomplete' },
		contributors: 2,
		last_report: '2026-11-04T10:30:00Z'
	};

	it('names how many contributed and when last, and words each figure: a partial with "+", a null by its reason', () => {
		const t = teamView(ticketTeam, TICKET_TEAM);
		expect(t.line).toBe('from 2 contributors · last report Nov 4, 10:30 UTC');
		const text = Object.fromEntries(t.figures.map((f) => [f.key, f.text]));
		expect(text.worked_minutes).toBe('1h 30m');
		expect(text.ai_token_cost).toBe('$3.00+');
		expect(text.cost_per_prompt_cycle).toBe('Cost is incomplete');
		expect(text.total_defects).toBe('0');
		expect(text.build_failure_rate).toBe('50%');
	});

	it('never shows a cost at the contributors’ own rates (B16)', () => {
		const keys = teamView(ticketTeam, TICKET_TEAM).figures.map((f) => f.key);
		expect(keys).not.toContain('effort_cost');
		expect(keys).not.toContain('rework_cost');
	});

	it('an AI cost some contributor could not fully price ends in "+"', () => {
		const t = teamView({ ...ticketTeam, team: { ...ticketTeam.team, values: { ai_token_cost: 3, api_cost_partial: 1 }, partial: {} } }, TICKET_TEAM);
		expect(t.figures.find((f) => f.key === 'ai_token_cost')!.text).toBe('$3.00+');
	});

	it('one contributor shows no figure even when a merge is handed over; nobody, no line at all', () => {
		expect(teamView({ ...ticketTeam, contributors: 1 }, TICKET_TEAM)).toEqual({ line: 'one contributor', figures: [] });
		expect(teamView({ ...noTeam, contributors: 1 }, TICKET_TEAM)).toEqual({ line: 'one contributor', figures: [] });
		expect(teamView(noTeam, TICKET_TEAM)).toEqual({ line: null, figures: [] });
	});

	it('a sprint shows the team’s AI time and API cost; a board its tokens and prompts per ticket', () => {
		const w = wire();
		w.boards[0].team = { values: { tokens_per_ticket: 1500, prompts_per_ticket: 2.5 }, reasons: {}, partial: {} };
		w.boards[0].contributors = 3;
		w.boards[0].last_report = '2026-11-04T10:30:00Z';
		w.boards[0].sprints[0].team = { values: { active_secs: 5400, api_cost: 1.24, api_cost_partial: 1 }, reasons: {}, partial: {} };
		w.boards[0].sprints[0].contributors = 2;
		const b = toDeliveryView(w).boards[0];
		expect(b.team.figures.map((f) => f.text)).toEqual(['1.5k', '2.5']);
		expect(b.sprints[0].team.figures.map((f) => f.text)).toEqual(['1h 30m', '$1.24+']);
		expect(b.sprints[1].team).toEqual({ line: null, figures: [] });
	});
});
