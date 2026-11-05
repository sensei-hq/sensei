// The dōjō Delivery page as a person meets it (tracker-delivery-metrics D5; #352):
// each board's sprints with their seven figures worded, a sprint's tickets by title,
// and a failure that never passes for "no boards".
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrWorkspaceDelivery from './ScrWorkspaceDelivery.svelte';
import { DeliveryPageState } from '$lib/delivery-state.svelte';
import { toDeliveryView } from '$lib/workspace-delivery-view';

afterEach(cleanup);

const noTeam = { team: null, team_words: null, contributors: 0, last_report: null };
const twoReported = { contributors: 2, team_words: null, last_report: '2026-11-04T10:30:00Z' };

const view = toDeliveryView({
	boards: [
		{
			board_id: 'b1',
			board_key: 'ab'.repeat(32),
			kind: 'jira',
			name: 'Mobile Scrum',
			predictability: null,
			predictability_reason: 'too_few_sprints',
			predictability_words: 'Too few sprints',
			history_at: null,
			pending_tickets: 0,
			reported_at: '2026-11-04T10:00:00Z',
			...noTeam,
			sprints: [
				{
					external_id: '101',
					seq: 101,
					name: 'Sprint 1',
					state: 'closed',
					start_at: '2026-10-01T00:00:00Z',
					end_at: '2026-10-14T00:00:00Z',
					complete_at: null,
					sdlc: {
						commitment_reliability: 0,
						total_completion: 50,
						scope_change: 10,
						carry_over_rate: 20,
						unplanned_work_ratio: 0,
						velocity: 8,
						points_commitment_reliability: null,
						facts: null,
						reasons: { points_commitment_reliability: 'history_pending' }
					},
					reason_words: { history_pending: 'Ticket history not read yet' },
					...twoReported,
					team: { values: { active_secs: 7200, api_cost: 4.5, api_cost_partial: 0 }, reasons: {}, partial: {} }
				}
			]
		}
	]
});

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('ScrWorkspaceDelivery', () => {
	it("shows each board's predictability words and each sprint's seven figures", () => {
		const { container, getByText } = render(ScrWorkspaceDelivery, {
			props: { workspaceName: 'API', view, error: null, state: new DeliveryPageState(async () => []) }
		});
		expect(getByText('Mobile Scrum')).toBeTruthy();
		expect(container.querySelector('[data-sdlc="predictability"]')!.textContent).toBe('Too few sprints');
		const row = container.querySelector('[data-sprint="101"]')!;
		expect(row.querySelectorAll('[data-sdlc]').length).toBe(9); // the seven figures, then the team's AI time and cost
		expect(row.querySelector('[data-sdlc="commitment_reliability"]')!.textContent).toBe('0%');
		expect(row.querySelector('[data-sdlc="points_commitment_reliability"]')!.textContent).toBe('Ticket history not read yet');
		expect(container.textContent).not.toMatch(/history_pending|too_few_sprints/);
	});

	it("selecting a sprint lists its tickets by key and title", async () => {
		const state = new DeliveryPageState(async () => [
			{ key: 'AUTH-2', title: 'Login', status_category: null, points: null, sprints: ['101'], issue_created_at: null, ...noTeam }
		]);
		const { container, getByText } = render(ScrWorkspaceDelivery, { props: { workspaceName: 'API', view, error: null, state } });
		await fireEvent.click(container.querySelector('[data-sprint="101"] [data-action="select"]')!);
		await tick();
		expect(getByText('Login')).toBeTruthy();
		expect(container.querySelector('[data-ticket="AUTH-2"]')).toBeTruthy();
	});

	it("a sprint row shows the team's AI time and cost; one contributor shows none (B16)", () => {
		const { container } = render(ScrWorkspaceDelivery, {
			props: { workspaceName: 'API', view, error: null, state: new DeliveryPageState(async () => []) }
		});
		const row = container.querySelector('[data-sprint="101"]')!;
		expect(row.querySelector('[data-sdlc="active_secs"]')!.textContent).toBe('2h');
		expect(row.querySelector('[data-sdlc="api_cost"]')!.textContent).toBe('$4.50');
		expect(container.querySelector('[data-team="project"]')).toBeNull();
	});

	it('gate 18: a ticket shows its title and "from 2 contributors", and opens to its merged figures', async () => {
		const state = new DeliveryPageState(async () => [
			{
				key: 'T-1', title: 'Checkout', status_category: null, points: null, sprints: ['101'], issue_created_at: null,
				...twoReported,
				team: { values: { worked_minutes: 90, build_failure_rate: 50, effort_cost: 120 }, reasons: {}, partial: {} }
			},
			{ key: 'T-2', title: 'Alone', status_category: null, points: null, sprints: ['101'], issue_created_at: null, ...noTeam, contributors: 1 }
		]);
		const { container, getByText } = render(ScrWorkspaceDelivery, { props: { workspaceName: 'API', view, error: null, state } });
		await fireEvent.click(container.querySelector('[data-sprint="101"] [data-action="select"]')!);
		await tick();
		expect(getByText('Checkout')).toBeTruthy();
		const t1 = container.querySelector('[data-ticket="T-1"]')!;
		expect(t1.querySelector('[data-team-line]')!.textContent).toMatch(/^from 2 contributors/);
		expect(t1.querySelector('[data-sdlc="worked_minutes"]')!.textContent).toBe('1h 30m');
		expect(t1.querySelector('[data-sdlc="effort_cost"]')).toBeNull();
		const t2 = container.querySelector('[data-ticket="T-2"]')!;
		expect(t2.querySelector('[data-team-line]')!.textContent).toBe('one contributor');
		expect(t2.querySelector('[data-team="ticket"]')).toBeNull();
	});

	it('a failed tickets read is an alert', async () => {
		const state = new DeliveryPageState(async () => {
			throw new Error('request failed (500)');
		});
		const { container } = render(ScrWorkspaceDelivery, { props: { workspaceName: 'API', view, error: null, state } });
		await fireEvent.click(container.querySelector('[data-sprint="101"] [data-action="select"]')!);
		await tick();
		expect(container.querySelector('[role="alert"]')!.textContent).toContain('request failed (500)');
	});

	it('a failed read is an error, not "no boards"', () => {
		const { container } = render(ScrWorkspaceDelivery, {
			props: { workspaceName: 'API', view: null, error: 'request failed (403)', state: new DeliveryPageState(async () => []) }
		});
		expect(container.querySelector('[role="alert"]')!.textContent).toContain('request failed (403)');
		expect(container.textContent).not.toContain('No board has reported');
	});

	it('no boards is said plainly', () => {
		const { container } = render(ScrWorkspaceDelivery, {
			props: { workspaceName: 'API', view: { boards: [] }, error: null, state: new DeliveryPageState(async () => []) }
		});
		expect(container.textContent).toContain('No board has reported to this dōjō yet.');
	});
});
