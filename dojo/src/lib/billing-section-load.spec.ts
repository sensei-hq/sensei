// billing polish (reviews/Design/L4-billing-followup-r1.md L2): the org page's billing
// section, as a testable function. It runs the billing read and the caller's repositories
// read in PARALLEL, and groups that org's billing-hidden rows. A failed repositories read
// is its own error; the billing read stands on its own.
import { describe, it, expect, vi } from 'vitest';
import type { MyRepoWire } from './client-data';
import { noBilling } from './billing-map';

import { loadBillingSection } from './billing-section-load';

const repo = (id: string, org: string, billing_hidden: boolean): MyRepoWire => ({
	repository_id: id,
	repo_key: `github.com/${org}/${id}`,
	name: id,
	tenant: `github/${org}`,
	owning_org: org,
	forge_visibility: 'private',
	authority: 'organization',
	may_share: !billing_hidden,
	elected: true,
	sync_enabled: false,
	configurable_by_me: false,
	reason_code: null,
	reason: null,
	remedy: null,
	reason_actor: null,
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	billing_hidden
});
const billingOk = async () => ({ value: noBilling(), error: null });

describe('loadBillingSection (billing polish, L2)', () => {
	it('the billing read and this org\'s hidden group from the caller\'s repositories', async () => {
		const readRepos = vi.fn(async () => [repo('core', 'acme', true), repo('site', 'acme', false), repo('g', 'globex', true)]);
		const out = await loadBillingSection('acme', billingOk, readRepos);
		expect(readRepos).toHaveBeenCalledOnce();
		expect(out.billing).toEqual(noBilling());
		expect(out.billingError).toBeNull();
		expect(out.billingHidden).toEqual({ org: 'acme', count: 1, repos: ['core'] });
		expect(out.billingHiddenError).toBeNull();
	});

	// A page that stubs the read (`async () => []`) shows nothing hidden; a real read does.
	it('an empty read is no group; it is not an error', async () => {
		const out = await loadBillingSection('acme', billingOk, async () => []);
		expect(out.billingHidden).toBeNull();
		expect(out.billingHiddenError).toBeNull();
	});

	it('the two reads run in parallel: the repositories read starts before billing answers', async () => {
		let releaseBilling!: () => void;
		const billingGate = new Promise<void>((r) => (releaseBilling = r));
		const readBilling = vi.fn(async () => {
			await billingGate;
			return { value: noBilling(), error: null };
		});
		const readRepos = vi.fn(async () => [] as MyRepoWire[]);
		const pending = loadBillingSection('acme', readBilling, readRepos);
		await Promise.resolve();
		await Promise.resolve();
		expect(readRepos).toHaveBeenCalledOnce();
		releaseBilling();
		await pending;
	});

	it('a failed repositories read is its own error; billing stands', async () => {
		const out = await loadBillingSection('acme', billingOk, async () => {
			throw new Error('boom');
		});
		expect(out.billing).toEqual(noBilling());
		expect(out.billingError).toBeNull();
		expect(out.billingHidden).toBeNull();
		expect(out.billingHiddenError).toEqual({ status: 0, message: 'could not reach the dojo service' });
	});

	it('a failed billing read carries its error through', async () => {
		const err = { status: 403, message: 'admins only' };
		const out = await loadBillingSection('acme', async () => ({ value: noBilling(), error: err }), async () => []);
		expect(out.billingError).toEqual(err);
	});
});
