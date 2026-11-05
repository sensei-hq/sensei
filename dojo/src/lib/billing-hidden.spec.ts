// billing build · WP8 (reviews/Design/L4-billing-build-tests-r2.md H1; r3 L3, L0 rulings 2026-11-05):
// Billing's hidden-repositories section groups the rows the SERVER flagged
// (`billing_hidden`) per organization. One pure helper, in a neutral module both
// zones can import (`$lib/billing-hidden`). It reads the flag and takes no predicate: the
// rule lives once, in `isBillingHiddenRepo` ($lib/server/repositories.ts), and nothing here
// re-derives it.
import { describe, it, expect } from 'vitest';
import type { MyRepoWire } from './client-data';

import { splitBillingHidden, orgHidden, loadOrgHidden } from './billing-hidden';

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
	it('nothing flagged: every row shown, nothing hidden', () => {
		const rows = [wire('a', 'acme', false), wire('n', null, false)];
		expect(splitBillingHidden(rows)).toEqual({ shown: rows, hidden: [] });
	});
});

// billing follow-up (user rulings 2026-11-05, reviews/Design/L4-billing-build-impl-r1.md):
// members see NO line about hidden repositories, so nothing links to Plan & billing from
// Sharing; admins see the org's hidden private repositories, count AND list, in Billing.
// The group carries the repositories' names and no link. Still the flag, never a predicate.
describe('billing follow-up · the hidden group names its repositories, and links nowhere', () => {
	it('each org group carries its count and the hidden repositories\' names, in order, and no href', () => {
		const { shown, hidden } = splitBillingHidden([
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

	it('the flag decides the group: false + not_subscribed stays out, true + null is in', () => {
		const { hidden } = splitBillingHidden([wire('kept', 'acme', false, 'not_subscribed'), wire('gone', 'globex', true, null)]);
		expect(hidden).toEqual([{ org: 'globex', count: 1, repos: ['gone'] }]);
	});

	it('orgHidden is one org\'s group, or null when none of its repositories is hidden', () => {
		const rows = [wire('a', 'acme', true), wire('g', 'globex', true), wire('p', 'acme', false)];
		expect(orgHidden(rows, 'acme')).toEqual({ org: 'acme', count: 1, repos: ['a'] });
		expect(orgHidden(rows, 'initech')).toBeNull();
		expect(orgHidden([wire('p', 'acme', false)], 'acme')).toBeNull();
	});

	it('loadOrgHidden reads the caller\'s repositories and groups that org\'s', async () => {
		const read = async () => [wire('a', 'acme', true), wire('g', 'globex', true)];
		expect(await loadOrgHidden('acme', read)).toEqual({ hidden: { org: 'acme', count: 1, repos: ['a'] }, error: null });
	});

	// A failed read is an error, never "nothing hidden" (D83).
	it('a failed read is the error, never an empty group', async () => {
		const read = async (): Promise<MyRepoWire[]> => {
			throw new Error('boom');
		};
		const out = await loadOrgHidden('acme', read);
		expect(out.hidden).toBeNull();
		expect(out.error).toEqual({ status: 0, message: 'could not reach the dojo service' });
	});
});

// billing polish (L4): a person reads the list, so it is in name order, not the read's uuid order.
describe('billing polish · the hidden list reads in name order', () => {
	it('an org group lists its repositories sorted by name', () => {
		const { hidden } = splitBillingHidden([wire('zeta', 'acme', true), wire('alpha', 'acme', true), wire('mid', 'acme', true)]);
		expect(hidden).toEqual([{ org: 'acme', count: 3, repos: ['alpha', 'mid', 'zeta'] }]);
	});
});
