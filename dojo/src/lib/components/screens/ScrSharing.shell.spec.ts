// billing build · ScrSharing on rokkit's ScreenShell (D87; reviews/Design/L4-billing-build-tests-r2.md
// L0 rulings 2026-11-05: Sharing adopts the shell in this item, view id dojo.you.sharing).
// The shell is layout only (CONTRACT §9): every sharing.spec.ts test stays green unedited;
// these pin where the screen's parts now sit, and that the shell is rokkit's own.
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { tick } from 'svelte';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import ScrSharing from './ScrSharing.svelte';
import { NAV_YOU } from '$lib/nav';
import type { MyRepoWire } from '$lib/client-data';

const repo = (over: Partial<MyRepoWire> = {}): MyRepoWire => ({
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
	billing_hidden: false,
	...over
});
const ROWS = [
	repo({ repository_id: 'a', name: 'api' }),
	repo({ repository_id: 's', name: 'site', repo_key: 'github.com/acme/site' }),
	repo({ repository_id: 'n', name: 'notes', repo_key: 'github.com/jdoe/notes', tenant: 'github/jdoe', owning_org: null })
];
const listed = (c: HTMLElement) => [...c.querySelectorAll('[aria-label^="Share "]')].map((e) => e.getAttribute('aria-label'));
const search = async (c: HTMLElement, q: string) => {
	const input = c.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement | null;
	if (!input) throw new Error('no search field in the shell filter band');
	await fireEvent.input(input, { target: { value: q } });
	await new Promise((r) => setTimeout(r, 300)); // the shell's 250 ms debounce
	await tick();
};
const sharingKanji = NAV_YOU.flatMap((g) => g.items).find((i) => i.id === 'sharing')!.kanji;

describe('ScrSharing · the screen shell (D87)', () => {
	afterEach(cleanup);

	it('renders through ScreenShell with view id dojo.you.sharing: the nav mark, title, and the description it has today', () => {
		const { container } = render(ScrSharing, { props: { repos: ROWS } });
		const shell = container.querySelector('[data-screen-shell]') as HTMLElement;
		expect(shell.dataset.view).toBe('dojo.you.sharing');
		expect(shell.querySelector('[data-screen-mark]')?.textContent).toBe(sharingKanji);
		expect(shell.querySelector('[data-screen-title]')?.textContent).toBe('Sharing');
		expect(shell.querySelector('[data-screen-description]')?.textContent).toMatch(/Which repositories send metrics to a dōjō/);
	});

	it('search sits in the filter band and narrows the list by name', async () => {
		const { container } = render(ScrSharing, { props: { repos: ROWS } });
		const input = container.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement;
		expect(input.placeholder).toBe('Search repositories');
		await search(container, 'notes');
		expect(listed(container)).toEqual(['Share notes']);
	});

	it('search matches the repository key too', async () => {
		const { container } = render(ScrSharing, { props: { repos: ROWS } });
		await search(container, 'acme/site');
		expect(listed(container)).toEqual(['Share site']);
	});

	// A search that matches nothing says so, rather than an empty list that reads as "no repositories".
	it('a search with no match says so, not "No repositories yet"', async () => {
		const { container } = render(ScrSharing, { props: { repos: ROWS } });
		await search(container, 'zzz');
		expect(listed(container)).toEqual([]);
		expect(container.querySelector('[data-screen-body]')?.textContent).toContain('No repository matches “zzz”.');
		expect(container.textContent).not.toMatch(/No repositories yet/);
	});

	// billing follow-up: search over hidden rows neither lists them nor says anything about them.
	it('search never lists a billing-hidden row, and no plan line appears', async () => {
		const hidden = repo({ repository_id: 'h', name: 'api-private', forge_visibility: 'private', authority: 'organization', billing_hidden: true, sync_enabled: false, reason_code: 'not_subscribed' });
		const { container } = render(ScrSharing, { props: { repos: [...ROWS, hidden] } });
		await search(container, 'api');
		expect(listed(container)).toEqual(['Share api']);
		expect(container.textContent).not.toMatch(/paid plans?/i);
		expect(container.querySelector('a[href*="/billing"]')).toBeNull();
	});

	// Parity (CONTRACT §9): the error and empty states move into the body unchanged.
	it('a failed read is the error state inside the shell body, never an empty list', () => {
		const { container } = render(ScrSharing, { props: { repos: [], error: 'boom' } });
		expect(container.querySelector('[data-screen-body] [data-load-error]')).toBeTruthy();
		expect(container.textContent).not.toMatch(/No repositories yet/);
	});

	it('no repositories says so inside the shell body', () => {
		const { container } = render(ScrSharing, { props: { repos: [] } });
		expect(container.querySelector('[data-screen-body]')?.textContent).toMatch(/No repositories yet/);
	});

	// L3 (r2): the attributes alone pass for a hand-rolled div; the shell must be rokkit's.
	// L1 (r3): an unused import passes the import check alone — it must also render the shell.
	it('ScrSharing imports ScreenShell from @rokkit/app and renders it', () => {
		const src = readFileSync(join(process.cwd(), 'src/lib/components/screens/ScrSharing.svelte'), 'utf8');
		expect(src).toMatch(/import\s*\{[^}]*\bScreenShell\b[^}]*\}\s*from\s*['"]@rokkit\/app['"]/);
		expect(src).toMatch(/<ScreenShell\b/);
	});
});
