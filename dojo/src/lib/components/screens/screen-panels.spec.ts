// billing build · remembered panels reach the screens (D87; reviews/Design/L4-billing-build-tests-r3.md L2).
// view-states.spec.ts pins the store alone; these pin that each screen on the shell READS it
// (a stored closed band starts closed) and WRITES it (a toggle saves the view's whole
// PanelState under its view id, CONTRACT §4). A store nobody wires fails here.
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterAll, afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import type { MyRepoWire } from '$lib/client-data';
import ScrBilling from './ScrBilling.svelte';
import ScrSharing from './ScrSharing.svelte';
import { toKitBilling } from '$lib/billing-map';

// The store is seeded BEFORE any module loads (vi.hoisted runs ahead of the imports), so a
// store read once at load sees it as well as one read at render. In-memory, as glyphs.spec.ts:
// Node's experimental localStorage shadows jsdom's.
const { store, KEY } = vi.hoisted(() => {
	const KEY = 'sensei.dojo.views';
	const store = new Map<string, string>([
		[KEY, JSON.stringify({ 'dojo.org.billing': { info: false }, 'dojo.you.sharing': { filters: false } })]
	]);
	vi.stubGlobal('localStorage', {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v)
	});
	return { store, KEY };
});
const saved = (viewId: string) => (JSON.parse(store.get(KEY) ?? '{}') as Record<string, Record<string, boolean>>)[viewId];

const repo: MyRepoWire = {
	repository_id: 'r1',
	repo_key: 'github.com/acme/api',
	name: 'api',
	tenant: 'github/acme',
	owning_org: 'acme',
	forge_visibility: 'public',
	authority: 'user',
	may_share: true,
	elected: true,
	sync_enabled: true,
	configurable_by_me: true,
	reason_code: null,
	reason: null,
	remedy: null,
	reason_actor: null,
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	billing_hidden: false
};

describe('remembered panels reach the screens (D87)', () => {
	afterEach(cleanup);
	afterAll(() => vi.unstubAllGlobals());

	it('Plan & billing opens its info band as stored, and a toggle saves dojo.org.billing', async () => {
		const billing = toKitBilling({ account: null, usage: { seats_used: 0, total_active_seats: 0, billable_users: [] } });
		const { container, getByRole } = render(ScrBilling, { props: { orgName: 'acme', billing } });
		expect(container.querySelector('[data-screen-shell]')).toBeTruthy();
		expect(container.querySelector('[data-screen-info]')).toBeNull();
		await fireEvent.click(getByRole('button', { name: 'What it’s for' }));
		await tick();
		expect(container.querySelector('[data-screen-info]')).toBeTruthy();
		expect(saved('dojo.org.billing')?.info).toBe(true);
	});

	it('Sharing opens its filter band as stored, and a toggle saves dojo.you.sharing', async () => {
		const { container, getByRole } = render(ScrSharing, { props: { repos: [repo] } });
		expect(container.querySelector('[data-screen-shell]')).toBeTruthy();
		expect(container.querySelector('[data-screen-filters]')).toBeNull();
		await fireEvent.click(getByRole('button', { name: 'Filters' }));
		await tick();
		expect(container.querySelector('[data-screen-filters]')).toBeTruthy();
		expect(saved('dojo.you.sharing')?.filters).toBe(true);
	});
});
