// The Delivery page's wiring (tracker-delivery-metrics D5; #352): a sprint's tickets
// are read for this tenant, workspace, board and sprint, as the caller.
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ getWorkspaceDeliveryTickets: vi.fn() }));
vi.mock('$lib/workspace-delivery-data', () => ({ getWorkspaceDeliveryTickets: mocks.getWorkspaceDeliveryTickets }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const { default: Page } = await import('./+page.svelte');
const { toDeliveryView } = await import('$lib/workspace-delivery-view');

afterEach(cleanup);

const noTeam = { team: null, team_words: null, contributors: 0, last_report: null };

const view = toDeliveryView({
	boards: [
		{
			board_id: 'b1',
			board_key: 'ab'.repeat(32),
			kind: 'jira',
			name: 'Mobile Scrum',
			predictability: 12.5,
			predictability_reason: null,
			predictability_words: null,
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
					start_at: null,
					end_at: null,
					complete_at: null,
					sdlc: null,
					reason_words: null,
					...noTeam
				}
			]
		}
	]
});

describe('the Delivery page', () => {
	it("reads the selected sprint's tickets for this tenant, workspace and board, as the caller", async () => {
		mocks.getWorkspaceDeliveryTickets.mockResolvedValue([]);
		const { container } = render(Page, {
			props: {
				data: {
					slug: 'acme',
					orgName: 'Acme',
					tenantKey: 'organization/acme',
					accessToken: 'jwt-A',
					workspace: { id: 'w1', name: 'API' },
					deliveryView: view,
					deliveryError: null
				}
			} as never
		});
		await fireEvent.click(container.querySelector('[data-sprint="101"] [data-action="select"]')!);
		expect(mocks.getWorkspaceDeliveryTickets).toHaveBeenCalledWith('organization/acme', 'w1', 'b1', { sprint: '101' }, { accessToken: 'jwt-A' });
	});
});
