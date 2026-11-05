// billing build · WP8 (reviews/Design/L4-billing-build-tests-r2.md H1; r3 L3, L0 rulings 2026-11-05):
// Sharing's hidden line — and later Billing's member line (M4) — group the rows the SERVER
// flagged (`billing_hidden`) per organization. One pure helper, in a neutral module both
// zones can import (`$lib/billing-hidden`). It reads the flag and takes no predicate: the
// rule lives once, in `isBillingHiddenRepo` ($lib/server/repositories.ts), and nothing here
// re-derives it.
import { describe, it, expect } from 'vitest';
import type { MyRepoWire } from './client-data';

// ── test-local shape of the module the build adds; phase B imports it directly ──
type HiddenOrg = { org: string; count: number; href: string };
type BillingHidden = { splitBillingHidden(rows: MyRepoWire[]): { shown: MyRepoWire[]; hidden: HiddenOrg[] } };
const load = async () => (await import(/* @vite-ignore */ './billing-hidden' + '')) as unknown as BillingHidden;

const wire = (id: string, org: string | null, billing_hidden: boolean, reason_code: string | null = null): MyRepoWire => ({
	repository_id: id,
	repo_key: `github.com/${org ?? 'jdoe'}/${id}`,
	name: id,
	tenant: `github/${org ?? 'jdoe'}`,
	owning_org: org,
	forge_visibility: 'private',
	authority: org ? 'organization' : 'user',
	may_share: !billing_hidden,
	elected: true,
	sync_enabled: false,
	configurable_by_me: false,
	reason_code,
	reason: null,
	remedy: null,
	reason_actor: null,
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	billing_hidden
});

describe('splitBillingHidden · groups by the flag (WP8)', () => {
	it.skip('pending: billing-build — shown rows keep their order; flagged rows are counted per org, first seen first, linked to its Plan & billing', async () => {
		const { shown, hidden } = (await load()).splitBillingHidden([
			wire('a', 'acme', true),
			wire('p', 'acme', false),
			wire('g', 'globex', true),
			wire('b', 'acme', true),
			wire('n', null, false)
		]);
		expect(shown.map((r) => r.repository_id)).toEqual(['p', 'n']);
		expect(hidden).toEqual([
			{ org: 'acme', count: 2, href: '/org/acme/billing' },
			{ org: 'globex', count: 1, href: '/org/globex/billing' }
		]);
	});

	it.skip('pending: billing-build — the flag decides, never reason_code: false + not_subscribed is shown, true + null is hidden', async () => {
		const { shown, hidden } = (await load()).splitBillingHidden([wire('kept', 'acme', false, 'not_subscribed'), wire('gone', 'globex', true, null)]);
		expect(shown.map((r) => r.repository_id)).toEqual(['kept']);
		expect(hidden).toEqual([{ org: 'globex', count: 1, href: '/org/globex/billing' }]);
	});

	it.skip('pending: billing-build — nothing flagged: every row shown, no line', async () => {
		const rows = [wire('a', 'acme', false), wire('n', null, false)];
		expect((await load()).splitBillingHidden(rows)).toEqual({ shown: rows, hidden: [] });
	});
});
