// billing build · WP8's UI on Sharing (Design; handoff WP8, reviews/L3/wp8-tests-r1.md L0
// rulings). An organization's private repositories hidden by the plan leave the list;
// in their place, ONE line per organization — private repositories are supported on
// paid plans — linking to that organization's Plan & billing. Hidden, not deleted.
import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import ScrSharing from './ScrSharing.svelte';
import type { MyRepoWire } from '$lib/client-data';

// ── test-local wire shape (entitlement_refusal, L3 adds it to the read); phase B deletes it ──
type Row = MyRepoWire & { entitlement_refusal: string | null };
const repo = (over: Partial<Row> = {}): Row => ({
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
const show = (repos: Row[]) => render(ScrSharing, { props: { repos: repos as MyRepoWire[] } });
const listed = (c: HTMLElement) => [...c.querySelectorAll('[aria-label^="Share "]')].map((e) => e.getAttribute('aria-label'));

describe('ScrSharing · WP8', () => {
	afterEach(cleanup);

	it.skip('pending: billing-build — sharing_hides_an_org_private_repo_until_subscribed_and_links_to_billing', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'core' }),
			repo({ repository_id: 'b', name: 'portal', entitlement_refusal: 'unclaimed', reason_code: 'unclaimed' }),
			repo({ repository_id: 'p', name: 'site', forge_visibility: 'public', entitlement_refusal: null, reason_code: null, may_share: true, sync_enabled: true })
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

	it.skip('pending: billing-build — a lapsed subscription hides them too (D88: no grace)', () => {
		const { container } = show([repo({ entitlement_refusal: 'subscription_expired', reason_code: 'subscription_expired' })]);
		expect(listed(container)).toEqual([]);
		expect(container.querySelector('[data-plan-hidden="acme"]')).toBeTruthy();
	});

	it.skip('pending: billing-build — one line per organization, never one per repository', () => {
		const { container } = show([
			repo({ repository_id: 'a', name: 'a' }),
			repo({ repository_id: 'b', name: 'b' }),
			repo({ repository_id: 'c', name: 'c', tenant: 'github/globex', owning_org: 'globex' })
		]);
		expect([...container.querySelectorAll('[data-plan-hidden]')].map((e) => (e as HTMLElement).dataset.planHidden)).toEqual(['acme', 'globex']);
	});

	it.skip('pending: billing-build — sharing_lists_a_personal_private_repo', () => {
		const { container } = show([
			repo({ name: 'notes', tenant: 'github/jdoe', owning_org: null, authority: 'user', entitlement_refusal: null, reason_code: null, may_share: true, sync_enabled: true, configurable_by_me: true })
		]);
		expect(listed(container)).toEqual(['Share notes']);
		expect(container.querySelector('[data-plan-hidden]')).toBeNull();
	});

	it.skip('pending: billing-build — a capture refusal stays listed with its reason; it is not the plan\'s to hide', () => {
		const { container, getByText } = show([
			repo({ name: 'unknown-vis', forge_visibility: null, entitlement_refusal: 'forge_visibility_unknown', reason_code: 'forge_visibility_unknown', reason: 'Waiting to learn whether it is public or private' })
		]);
		expect(listed(container)).toEqual(['Share unknown-vis']);
		expect(getByText(/Waiting to learn whether it is public or private/)).toBeTruthy();
		expect(container.querySelector('[data-plan-hidden]')).toBeNull();
	});

	it.skip('pending: billing-build — when every repository is hidden, the line shows, not "No repositories yet"', () => {
		const { container } = show([repo()]);
		expect(container.textContent).not.toMatch(/No repositories yet/);
		expect(container.querySelector('[data-plan-hidden="acme"]')).toBeTruthy();
	});
});
