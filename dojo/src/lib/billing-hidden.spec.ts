// billing build · WP8 (reviews/Design/L4-billing-build-tests-r2.md H1; r3 L3, L0 rulings 2026-11-05):
// Sharing's hidden line — and later Billing's member line (M4) — group the rows the SERVER
// flagged (`billing_hidden`) per organization. One pure helper, in a neutral module both
// zones can import (`$lib/billing-hidden`). It reads the flag and takes no predicate: the
// rule lives once, in `isBillingHiddenRepo` ($lib/server/repositories.ts), and nothing here
// re-derives it.
import { describe, it, expect } from 'vitest';
import type { MyRepoWire } from './client-data';

import { splitBillingHidden } from './billing-hidden';
import * as hiddenModule from './billing-hidden';
import type { ReadError } from './read-error';

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
	it('shown rows keep their order; flagged rows are counted per org, first seen first, linked to its Plan & billing', () => {
		const { shown, hidden } = splitBillingHidden([
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

	it('the flag decides, never reason_code: false + not_subscribed is shown, true + null is hidden', () => {
		const { shown, hidden } = splitBillingHidden([wire('kept', 'acme', false, 'not_subscribed'), wire('gone', 'globex', true, null)]);
		expect(shown.map((r) => r.repository_id)).toEqual(['kept']);
		expect(hidden).toEqual([{ org: 'globex', count: 1, href: '/org/globex/billing' }]);
	});

	it('nothing flagged: every row shown, no line', () => {
		const rows = [wire('a', 'acme', false), wire('n', null, false)];
		expect(splitBillingHidden(rows)).toEqual({ shown: rows, hidden: [] });
	});
});

// billing follow-up (user rulings 2026-11-05, reviews/Design/L4-billing-build-impl-r1.md):
// members see NO line about hidden repositories, so nothing links to Plan & billing from
// Sharing; admins see the org's hidden private repositories, count AND list, in Billing.
// The group carries the repositories' names and no link. Still the flag, never a predicate.
// ── test-local shapes the follow-up adds; its implementation commit deletes them ──
type HiddenGroup = { org: string; count: number; repos: string[] };
type Followup = {
	orgHidden(rows: MyRepoWire[], org: string): HiddenGroup | null;
	loadOrgHidden(org: string, read: () => Promise<MyRepoWire[]>): Promise<{ hidden: HiddenGroup | null; error: ReadError | null }>;
};
const next = hiddenModule as unknown as Followup & { splitBillingHidden(rows: MyRepoWire[]): { shown: MyRepoWire[]; hidden: HiddenGroup[] } };

describe('billing follow-up · the hidden group names its repositories, and links nowhere', () => {
	it.skip('pending: billing-followup — each org group carries its count and the hidden repositories\' names, in order, and no href', () => {
		const { shown, hidden } = next.splitBillingHidden([
			wire('a', 'acme', true),
			wire('p', 'acme', false),
			wire('g', 'globex', true),
			wire('b', 'acme', true)
		]);
		expect(shown.map((r) => r.repository_id)).toEqual(['p']);
		expect(hidden).toEqual([
			{ org: 'acme', count: 2, repos: ['a', 'b'] },
			{ org: 'globex', count: 1, repos: ['g'] }
		]);
	});

	it.skip('pending: billing-followup — the flag decides the group: false + not_subscribed stays out, true + null is in', () => {
		const { hidden } = next.splitBillingHidden([wire('kept', 'acme', false, 'not_subscribed'), wire('gone', 'globex', true, null)]);
		expect(hidden).toEqual([{ org: 'globex', count: 1, repos: ['gone'] }]);
	});

	it.skip('pending: billing-followup — orgHidden is one org\'s group, or null when none of its repositories is hidden', () => {
		const rows = [wire('a', 'acme', true), wire('g', 'globex', true), wire('p', 'acme', false)];
		expect(next.orgHidden(rows, 'acme')).toEqual({ org: 'acme', count: 1, repos: ['a'] });
		expect(next.orgHidden(rows, 'initech')).toBeNull();
		expect(next.orgHidden([wire('p', 'acme', false)], 'acme')).toBeNull();
	});

	it.skip('pending: billing-followup — loadOrgHidden reads the caller\'s repositories and groups that org\'s', async () => {
		const read = async () => [wire('a', 'acme', true), wire('g', 'globex', true)];
		expect(await next.loadOrgHidden('acme', read)).toEqual({ hidden: { org: 'acme', count: 1, repos: ['a'] }, error: null });
	});

	// A failed read is an error, never "nothing hidden" (D83).
	it.skip('pending: billing-followup — a failed read is the error, never an empty group', async () => {
		const read = async (): Promise<MyRepoWire[]> => {
			throw new Error('boom');
		};
		const out = await next.loadOrgHidden('acme', read);
		expect(out.hidden).toBeNull();
		expect(out.error).toEqual({ status: 0, message: 'could not reach the dojo service' });
	});
});
