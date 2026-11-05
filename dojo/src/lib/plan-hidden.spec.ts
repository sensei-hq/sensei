// billing build · WP8's UI (Design; reviews/L3/wp8-tests-r1.md, L0 rulings 2026-11-04).
// One predicate decides which of the caller's repositories the plan hides (F5): an
// ORGANIZATION's private repository refused for its subscription — `not_subscribed`,
// `unclaimed`, or `subscription_expired` (D88: no grace). A capture refusal is not the
// plan's to hide; a personal dōjō's private repository is entitled and free (WP8).
// The same predicate feeds Sharing's hidden line and Billing's member-line count, so
// it lives in shared code (`$lib/plan-hidden`), which `$lib/server/repositories.ts`
// imports too; neither re-derives it. Hidden rows still reach the load.
import { describe, it, expect } from 'vitest';
import type { MyRepoWire } from './client-data';

// ── test-local shapes the build adds; phase B deletes these ──
type Row = MyRepoWire & { entitlement_refusal: string | null };
type HiddenOrg = { org: string; count: number; href: string };
type PlanHidden = {
	HIDDEN_BY_PLAN: readonly string[];
	hiddenByPlan(r: Row): boolean;
	splitByPlan(rows: Row[]): { shown: Row[]; hidden: HiddenOrg[] };
};
const load = async () => (await import(/* @vite-ignore */ './plan-hidden' + '')) as unknown as PlanHidden;

const row = (over: Partial<Row> = {}): Row => ({
	repository_id: 'r1',
	repo_key: 'github.com/acme/api',
	name: 'api',
	tenant: 'github/acme',
	owning_org: 'acme',
	forge_visibility: 'private',
	authority: 'organization',
	may_share: false,
	elected: true,
	sync_enabled: false,
	configurable_by_me: false,
	reason_code: 'not_subscribed',
	entitlement_refusal: 'not_subscribed',
	reason: 'No active subscription for this organization',
	remedy: 'Ask an admin to subscribe',
	reason_actor: 'organization',
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	...over
});

describe('plan-hidden · the one predicate (WP8, F5)', () => {
	it.skip('pending: billing-build — the hidden set is the three subscription refusals', async () => {
		expect([...(await load()).HIDDEN_BY_PLAN].sort()).toEqual(['not_subscribed', 'subscription_expired', 'unclaimed']);
	});

	it.skip.each(['not_subscribed', 'unclaimed', 'subscription_expired'])(
		'pending: billing-build — an org private repository refused %s is hidden',
		async (code) => {
			expect((await load()).hiddenByPlan(row({ entitlement_refusal: code, reason_code: code }))).toBe(true);
		}
	);

	it.skip.each([
		['a capture refusal', { entitlement_refusal: 'forge_visibility_unknown', reason_code: 'forge_visibility_unknown', forge_visibility: null }],
		['an entitled org private repository', { entitlement_refusal: null, reason_code: null, may_share: true, sync_enabled: true }],
		['an org public repository', { forge_visibility: 'public', entitlement_refusal: null, reason_code: null, may_share: true }],
		['a personal private repository (entitled, free)', { tenant: 'github/jdoe', owning_org: null, authority: 'user', entitlement_refusal: null, reason_code: 'not_elected_user', may_share: true }],
		['an election refusal, not the plan\'s', { entitlement_refusal: null, reason_code: 'not_elected_org', may_share: true }]
	] as const)('pending: billing-build — %s is not hidden', async (_label, over) => {
		expect((await load()).hiddenByPlan(row(over as Partial<Row>))).toBe(false);
	});

	it.skip('pending: billing-build — split: shown rows kept in order; hidden rows counted per org, linked to that org\'s Plan & billing', async () => {
		const rows = [
			row({ repository_id: 'a', repo_key: 'github.com/acme/a' }),
			row({ repository_id: 'p', repo_key: 'github.com/acme/site', forge_visibility: 'public', entitlement_refusal: null, reason_code: null, may_share: true }),
			row({ repository_id: 'b', repo_key: 'github.com/acme/b', entitlement_refusal: 'subscription_expired', reason_code: 'subscription_expired' }),
			row({ repository_id: 'c', repo_key: 'github.com/globex/c', tenant: 'github/globex', owning_org: 'globex', entitlement_refusal: 'unclaimed', reason_code: 'unclaimed' })
		];
		const { shown, hidden } = (await load()).splitByPlan(rows);
		expect(shown.map((r) => r.repository_id)).toEqual(['p']);
		expect(hidden).toEqual([
			{ org: 'acme', count: 2, href: '/org/acme/billing' },
			{ org: 'globex', count: 1, href: '/org/globex/billing' }
		]);
	});
});
