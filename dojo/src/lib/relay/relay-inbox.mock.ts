// Hand-crafted inbox mock — the mock half of the Load seam. Mirrors the mockup's
// inbox board (the five "needs you" rows summing to 8 need-you) so the rail is a
// 1:1 fidelity comparison, plus a stalled + a finished run to exercise the other
// tabs. Swap `loadRelayInbox`'s body to a live user-wide fetch later; this stays as
// the fixture. NOT random — each row targets a specific state.
import type { RelaySession, RelayPhase, SegmentState } from './types';

// Ages are relative to real now so the rail reads in live minutes ("2m", "38m")
// like the mockup — a fixed base would drift to hours as wall-clock moves past it.
const iso = (min: number) => new Date(Date.now() - min * 60_000).toISOString();

// Build N phases with the given per-phase states (drives the pip strip).
const phases = (states: SegmentState[]): RelayPhase[] =>
	states.map((state, i) => ({
		id: `p${i}`,
		title: `Phase ${i + 1}`,
		state,
		tasks: [{ id: `p${i}t0`, title: `task ${i}`, state }]
	}));

const ask = (
	id: string,
	action: RelaySession['asks'][number]['action'],
	prompt: string,
	opts: string[],
	extra: Partial<RelaySession['asks'][number]> = {}
) => ({
	id,
	action,
	blocking: true,
	prompt,
	options: opts,
	createdAt: iso(3),
	...extra
});

export function relayInboxMock(): RelaySession[] {
	return [
		{
			id: 'run-lumen',
			project: 'lumen-auth',
			title: 'refactor refresh-token rotation',
			goal: 'rotate refresh tokens without a client-visible logout',
			status: 'running',
			done: 5,
			total: 12,
			phase: 'Implement',
			model: 'claude-sonnet',
			startedAt: iso(38),
			lastEventAt: iso(2),
			needs: 3,
			attention: null,
			plan: { phases: phases(['done', 'done', 'active', 'pending', 'pending']) },
			asks: [
				ask('a1', 'approve', 'Run the staging migration?', ['Run it', 'Dry-run first', 'Skip the migration'], {
					context:
						'pnpm db:migrate --env=staging · touches an auth-boundary schema · the company rung requires a human yes',
					segmentId: 't11',
					taskTitle: 'Staging migration'
				}),
				ask('a2', 'choose', 'Which redaction sink?', ['Route to sink', 'Delete the lines'], {
					context: 'Two debug lines still print the refresh token.',
					segmentId: 't8',
					taskTitle: 'Scrub token logging'
				}),
				ask('a3', 'chat', 'Confirm the rollout window', [], {
					context: 'sensei will ship behind a flag; tell it when to flip.',
					segmentId: 't12',
					taskTitle: 'Rollout'
				})
			]
		},
		{
			id: 'run-agency',
			project: 'agency-monorepo',
			title: 'wire initech billing webhook',
			goal: 'deliver billing events to initech',
			status: 'running',
			done: 2,
			total: 6,
			phase: 'Implement',
			lastEventAt: iso(9),
			needs: 1,
			attention: null,
			plan: { phases: phases(['done', 'active', 'blocked', 'pending']) },
			asks: [ask('a4', 'approve', 'Send a test event to initech staging?', ['Send it', 'Skip'])]
		},
		{
			id: 'run-ledger',
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
			plan: { phases: phases(['done', 'done', 'done', 'active', 'pending']) },
			asks: [ask('a5', 'choose', 'Two debug lines still print the token. What happens?', ['Route to redaction sink', 'Delete the lines'])]
		},
		{
			id: 'run-globex',
			project: 'globex-portal',
			title: 'harden client webhook intake',
			goal: 'validate + rate-limit inbound client webhooks',
			status: 'running',
			done: 2,
			total: 4,
			phase: 'Implement',
			lastEventAt: iso(60),
			needs: 2,
			attention: null,
			plan: { phases: phases(['done', 'active', 'needs_review']) },
			asks: [
				ask('a6', 'approve', 'Enable the strict schema validator?', ['Enable', 'Advisory only']),
				ask('a7', 'chat', 'What is the per-tenant rate ceiling?', [])
			]
		},
		{
			id: 'run-gateway',
			project: 'api-gateway',
			title: 'split rate limiter per tenant',
			goal: 'per-tenant rate buckets',
			status: 'running',
			done: 2,
			total: 4,
			phase: 'Implement',
			lastEventAt: iso(47),
			needs: 1,
			attention: null,
			plan: { phases: phases(['done', 'active', 'pending']) },
			asks: [ask('a8', 'resume', 'Resume after the limit reset?', ['Resume now', 'Wait'])]
		},
		{
			id: 'run-stalled',
			project: 'notify-worker',
			title: 'migrate the digest scheduler',
			goal: null,
			status: 'stalled',
			done: 1,
			total: 4,
			phase: 'Implement',
			lastEventAt: iso(180),
			needs: 0,
			attention: 'stalled',
			plan: { phases: phases(['done', 'active', 'pending', 'pending']) },
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
			plan: { phases: phases(['done', 'done', 'done']) },
			asks: []
		}
	];
}
