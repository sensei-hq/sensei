// billing build · WP8's UI on Sharing (Design; reviews/L3/wp8-tests-r2.md L0 rulings,
// reviews/Design/L4-billing-build-tests-r2.md H1). The server decides which rows the plan
// hides and sends the answer as ONE flag, `MyRepoWire.billing_hidden`
// (`isBillingHiddenRepo` in `$lib/server/repositories.ts`, pinned in repositories.spec.ts).
// The screen reads that flag and re-derives nothing: a flagged row leaves the list, and in
// its place ONE line per organization says private repositories are supported on paid
// plans and links to that organization's Plan & billing. Hidden, not deleted.
import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import ScrSharing from './ScrSharing.svelte';
import type { MyRepoWire } from '$lib/client-data';

// The real wire shape: an org private repository the server flagged hidden.
const repo = (over: Partial<MyRepoWire> = {}): MyRepoWire => ({
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
	reason: 'No active subscription for this organization',
	remedy: 'Ask an admin to subscribe',
	reason_actor: 'organization',
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	billing_hidden: true,
	...over
});
const shown = (over: Partial<MyRepoWire> = {}) => repo({ billing_hidden: false, ...over });
const show = (repos: MyRepoWire[]) => render(ScrSharing, { props: { repos } });
const listed = (c: HTMLElement) => [...c.querySelectorAll('[aria-label^="Share "]')].map((e) => e.getAttribute('aria-label'));
const lines = (c: HTMLElement) => [...c.querySelectorAll('[data-plan-hidden]')].map((e) => (e as HTMLElement).dataset.planHidden);

describe('ScrSharing · WP8 · reads billing_hidden', () => {
	afterEach(cleanup);

	it.skip('pending: billing-build — sharing_hides_an_org_private_repo_until_subscribed_and_links_to_billing', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'core' }),
			repo({ repository_id: 'b', name: 'portal', reason_code: 'unclaimed' }),
			shown({ repository_id: 'p', name: 'site', forge_visibility: 'public', reason_code: null, reason: null, remedy: null, may_share: true, sync_enabled: true })
		]);
		expect(listed(container)).toEqual(['Share site']);
		expect(container.textContent).not.toMatch(/\bcore\b|\bportal\b/);
		const line = container.querySelector('[data-plan-hidden="acme"]') as HTMLElement;
		expect(line.textContent).toMatch(/acme/);
		expect(line.textContent).toMatch(/Private repositories are supported on paid plans/);
		const link = line.querySelector('a') as HTMLAnchorElement;
		expect(link.getAttribute('href')).toBe('/org/acme/billing');
		expect(link.textContent?.trim()).toBe('Plan & billing');
	});

	// The flag is the whole decision: a flagged row is hidden whatever reason_code says.
	it.skip('pending: billing-build — billing_hidden true hides the row whatever reason_code says', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'a', reason_code: null, reason: null, remedy: null }),
			repo({ repository_id: 'b', name: 'b', reason_code: 'not_elected_org' })
		]);
		expect(listed(container)).toEqual([]);
		expect(lines(container)).toEqual(['acme']);
	});

	// A guard (passes today, must stay): it kills a screen that re-derives the rule from
	// reason_code, because the server said "not hidden".
	it.skip('pending: billing-build — billing_hidden false with reason_code not_subscribed stays listed, with its reason', () => {
		const { container, getByText } = show([shown({ name: 'kept', reason_code: 'not_subscribed' })]);
		expect(listed(container)).toEqual(['Share kept']);
		expect(getByText(/No active subscription for this organization/)).toBeTruthy();
		expect(container.querySelector('[data-plan-hidden]')).toBeNull();
	});

	it.skip('pending: billing-build — one line per organization, never one per repository', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'a' }),
			repo({ repository_id: 'b', name: 'b' }),
			repo({ repository_id: 'c', name: 'c', repo_key: 'github.com/globex/c', tenant: 'github/globex', owning_org: 'globex' })
		]);
		expect(lines(container)).toEqual(['acme', 'globex']);
		expect(container.querySelector('[data-plan-hidden="globex"] a')?.getAttribute('href')).toBe('/org/globex/billing');
	});

	it.skip('pending: billing-build — when every repository is hidden, the line shows, not "No repositories yet"', () => {
		const { container } = show([repo()]);
		expect(container.textContent).not.toMatch(/No repositories yet/);
		expect(lines(container)).toEqual(['acme']);
	});

	// Guards (pass today, must stay): rows the server did not flag are listed.
	it.skip('pending: billing-build — sharing_lists_a_personal_private_repo', () => {
		const { container } = show([
			shown({ name: 'notes', repo_key: 'github.com/jdoe/notes', tenant: 'github/jdoe', owning_org: null, authority: 'user', reason_code: null, reason: null, remedy: null, may_share: true, sync_enabled: true, configurable_by_me: true })
		]);
		expect(listed(container)).toEqual(['Share notes']);
		expect(container.querySelector('[data-plan-hidden]')).toBeNull();
	});

	it.skip("pending: billing-build — a capture refusal stays listed with its reason; it is not the plan's to hide", () => {
		const { container, getByText } = show([
			shown({ name: 'unknown-vis', forge_visibility: null, reason_code: 'forge_visibility_unknown', reason: 'Waiting to learn whether it is public or private' })
		]);
		expect(listed(container)).toEqual(['Share unknown-vis']);
		expect(getByText(/Waiting to learn whether it is public or private/)).toBeTruthy();
		expect(container.querySelector('[data-plan-hidden]')).toBeNull();
	});
});
