// billing build · WP8's UI on Sharing (Design; reviews/L3/wp8-tests-r2.md L0 rulings,
// reviews/Design/L4-billing-build-tests-r2.md H1). The server decides which rows the plan
// hides and sends the answer as ONE flag, `MyRepoWire.billing_hidden`
// (`isBillingHiddenRepo` in `$lib/server/repositories.ts`, pinned in repositories.spec.ts).
// The screen reads that flag and re-derives nothing: a flagged row leaves the list for
// every user, and nothing on Sharing says why (user ruling 2026-11-05) — admins see the
// hidden repositories in their org's Plan & billing.
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
// Nothing on Sharing speaks of hidden repositories: no plan claim, no link to Plan & billing.
const saysNothing = (c: HTMLElement) => !/paid plans?/i.test(c.textContent ?? '') && c.querySelector('a[href*="/billing"]') === null;

describe('ScrSharing · WP8 · reads billing_hidden', () => {
	afterEach(cleanup);

	// The flag is the whole decision: a flagged row is hidden whatever reason_code says.
	it('billing_hidden true hides the row whatever reason_code says', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'a', reason_code: null, reason: null, remedy: null }),
			repo({ repository_id: 'b', name: 'b', reason_code: 'not_elected_org' })
		]);
		expect(listed(container)).toEqual([]);
		expect(saysNothing(container)).toBe(true);
	});

	// A guard (passes today, must stay): it kills a screen that re-derives the rule from
	// reason_code, because the server said "not hidden".
	it('billing_hidden false with reason_code not_subscribed stays listed, with its reason', () => {
		const { container, getByText } = show([shown({ name: 'kept', reason_code: 'not_subscribed' })]);
		expect(listed(container)).toEqual(['Share kept']);
		expect(getByText(/No active subscription for this organization/)).toBeTruthy();
		expect(saysNothing(container)).toBe(true);
	});

	// Guards (pass today, must stay): rows the server did not flag are listed.
	it('sharing_lists_a_personal_private_repo', () => {
		const { container } = show([
			shown({ name: 'notes', repo_key: 'github.com/jdoe/notes', tenant: 'github/jdoe', owning_org: null, authority: 'user', reason_code: null, reason: null, remedy: null, may_share: true, sync_enabled: true, configurable_by_me: true })
		]);
		expect(listed(container)).toEqual(['Share notes']);
		expect(saysNothing(container)).toBe(true);
	});

	it("a capture refusal stays listed with its reason; it is not the plan's to hide", () => {
		const { container, getByText } = show([
			shown({ name: 'unknown-vis', forge_visibility: null, reason_code: 'forge_visibility_unknown', reason: 'Waiting to learn whether it is public or private' })
		]);
		expect(listed(container)).toEqual(['Share unknown-vis']);
		expect(getByText(/Waiting to learn whether it is public or private/)).toBeTruthy();
		expect(saysNothing(container)).toBe(true);
	});
});

// billing follow-up (user rulings 2026-11-05): a flagged repository is hidden for every
// user, and members see NO line about it — no "supported on paid plans" claim (wrong for a
// past-due org that IS on Pro) and no link to Plan & billing (an admin page a member can't
// open). Admins see the hidden repositories in Billing instead.
describe('ScrSharing · billing follow-up · no line about hidden repositories', () => {
	afterEach(cleanup);

	it('flagged rows leave the list with no plan line and no link to Plan & billing', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'core' }),
			repo({ repository_id: 'b', name: 'portal', repo_key: 'github.com/globex/portal', tenant: 'github/globex', owning_org: 'globex' }),
			shown({ repository_id: 'p', name: 'site', forge_visibility: 'public', reason_code: null, reason: null, remedy: null, may_share: true, sync_enabled: true })
		]);
		expect(listed(container)).toEqual(['Share site']);
		expect(container.textContent).not.toMatch(/\bcore\b|\bportal\b/);
		expect(saysNothing(container)).toBe(true);
		expect(container.textContent).not.toMatch(/paid plans?/i);
		expect(container.querySelector('a[href*="/billing"]')).toBeNull();
	});

	// When every repository is hidden the screen says there is nothing to share — not "No
	// repositories yet" (they exist) and nothing about plans.
	it('every row hidden: "none can be shared right now", no plan claim, not "No repositories yet"', () => {
		const { container } = show([repo(), repo({ repository_id: 'b', name: 'b' })]);
		expect(listed(container)).toEqual([]);
		expect(container.querySelector('[data-screen-body]')?.textContent).toContain('None of your repositories can be shared right now.');
		expect(container.textContent).not.toMatch(/No repositories yet|paid plans?/i);
		expect(container.querySelector('a[href*="/billing"]')).toBeNull();
	});
});
