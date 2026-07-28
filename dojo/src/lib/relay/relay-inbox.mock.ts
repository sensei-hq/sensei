// Hand-crafted inbox mock — the mock half of the Load seam. Deliberately exercises
// every UI state (needs-you gate · running with pips · stalled/no-heartbeat ·
// finished · empty plan) so the components + fidelity are built against real
// variety before the `/v1` read exists. Swap `loadRelayInbox`'s body to a live
// user-wide fetch later; this stays as the test fixture. NOT random — each row
// targets a specific state.
import type { RelaySession } from './types';

const iso = (min: number) => new Date(Date.UTC(2026, 6, 28, 12, 0) - min * 60_000).toISOString();

export function relayInboxMock(): RelaySession[] {
	return [
		{
			id: 'run-approve',
			project: 'lumen-auth',
			title: 'refactor refresh-token rotation',
			goal: 'rotate refresh tokens without a client-visible logout',
			status: 'running',
			done: 5,
			total: 12,
			phase: 'Implement',
			lastEventAt: iso(2),
			needs: 1,
			attention: null,
			plan: {
				phases: [
					{ id: 'p1', title: 'Design', state: 'done', tasks: [{ id: 't1', title: 'Token model', state: 'done' }] },
					{ id: 'p2', title: 'Implement', state: 'active', tasks: [
						{ id: 't2', title: 'Rotation endpoint', state: 'active', agent: 'builder', model: 'claude-sonnet' },
						{ id: 't3', title: 'Staging migration', state: 'blocked', specRef: 'spec/auth.md#t11' }
					] },
					{ id: 'p3', title: 'Ship', state: 'pending', tasks: [{ id: 't4', title: 'Cut release', state: 'pending' }] }
				]
			},
			asks: [
				{
					id: 'ask-1',
					action: 'approve',
					blocking: true,
					prompt: 'Run the staging migration?',
					context: 'pnpm db:migrate --env=staging · touches an auth-boundary schema',
					options: ['Run it', 'Dry-run first', 'Skip the migration'],
					segmentId: 't3',
					taskTitle: 'Staging migration',
					createdAt: iso(3)
				}
			]
		},
		{
			id: 'run-choose',
			project: 'ledger-core',
			title: 'add idempotency keys to ledger writes',
			goal: 'make ledger writes idempotent under retry',
			status: 'running',
			done: 6,
			total: 10,
			phase: 'Implement',
			lastEventAt: iso(6),
			needs: 1,
			attention: null,
			plan: { phases: [{ id: 'p1', title: 'Implement', state: 'active', tasks: [{ id: 't1', title: 'Key scheme', state: 'needs_review' }] }] },
			asks: [
				{
					id: 'ask-2',
					action: 'choose',
					blocking: true,
					prompt: 'Two debug lines still print the refresh token. What should happen?',
					context: 'company rule forbids logging tokens',
					options: ['Route to the redaction sink', 'Delete the lines'],
					segmentId: 't1',
					taskTitle: 'Route debug line',
					createdAt: iso(11)
				}
			]
		},
		{
			id: 'run-stalled',
			project: 'api-gateway',
			title: 'split rate limiter per tenant',
			goal: null,
			status: 'stalled',
			done: 2,
			total: 4,
			phase: 'Implement',
			lastEventAt: iso(47),
			needs: 0,
			attention: 'stalled',
			plan: { phases: [{ id: 'p1', title: 'Implement', state: 'active', tasks: [{ id: 't1', title: 'Per-tenant buckets', state: 'active' }] }] },
			asks: []
		},
		{
			id: 'run-done',
			project: 'lumen-auth',
			title: 'upgrade argon2 params',
			goal: 'raise the hash cost to the 2026 baseline',
			status: 'done',
			done: 3,
			total: 3,
			phase: 'Ship',
			lastEventAt: iso(1440),
			needs: 0,
			attention: null,
			plan: { phases: [{ id: 'p1', title: 'Ship', state: 'done', tasks: [{ id: 't1', title: 'Bump params', state: 'done' }] }] },
			asks: []
		}
	];
}
