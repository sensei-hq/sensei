// #299 — the sync files a repository under its sensei workspace, and the org
// Workspaces screen shows it to whoever can read that repository.
//
// openspec/changes/verify-dojo-sync-files-workspaces, tasks 2.2–2.6. Each sync
// cycle is the daemon's registration call, `POST /v1/you/repositories`, with the
// body `dojo_sync` sends (crates/senseid/src/tasks/dojo_sync.rs, `RepoInput`) and
// the signed-in person's session as the bearer, as the daemon presents it.
//
// The reader of the positive case is a NON-admin (`contributor`) holding a role
// on the filed repository: an admin reads every workspace of its dōjō through
// `administers_workspace`, so it would pass even with the repository read path
// broken. The negative reader is the dōjō's `lead` with no role on it.
import { test, expect, type Page } from '@playwright/test';
import {
	DOJO_URL,
	accessToken,
	createUser,
	localDojo,
	magicLink,
	ok,
	removeFixtures,
	type LocalDojo,
	type TestUser
} from './local-dojo';

const PREFIX = 'zz299e2e';
const run = Date.now().toString(36);
const org = `${PREFIX}-${run}`;
const unconnectedOrg = `${PREFIX}-none-${run}`;
const workspace = { slug: `${PREFIX}-ws-${run}`, name: `${PREFIX} workspace ${run}` };
const filedKey = `github.com/${org}/api`;
const unmappedKey = `github.com/${unconnectedOrg}/tool`;

type Registered = {
	mapped: { repo_key: string; repo_id: string; workspace_id?: string; workspace_error?: string }[];
	unmapped: { repo_key: string; reason: string }[];
};

let dojo: LocalDojo;
let tenantId: string;
let rho: TestUser; // contributor, syncs the repository and holds a role on it
let lex: TestUser; // lead, no role on the repository

/** One `dojo_sync` registration, as the daemon sends it. */
async function syncCycle(user: TestUser): Promise<Registered> {
	const token = await accessToken(dojo, user.email);
	const res = await fetch(`${DOJO_URL}/v1/you/repositories`, {
		method: 'POST',
		headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
		body: JSON.stringify({
			repos: [
				{ repo_key: filedKey, remote_url: `https://${filedKey}.git`, name: 'api', workspace },
				{ repo_key: unmappedKey, remote_url: `https://${unmappedKey}.git`, name: 'tool', workspace }
			]
		})
	});
	expect(res.status, await res.clone().text()).toBe(200);
	return (await res.json()) as Registered;
}

async function signIn(page: Page, user: TestUser): Promise<void> {
	await page.goto(await magicLink(dojo, user.email, `/org/${tenantId}/workspaces`));
	await page.waitForURL((u) => !u.pathname.startsWith('/auth') && !u.hash.includes('access_token'));
	// Sign-in provisions the personal dōjō in the background; let it finish so
	// the cleanup sees what it created.
	await page.waitForLoadState('networkidle');
}

async function openWorkspaces(page: Page): Promise<void> {
	await page.goto(`/org/${tenantId}/workspaces`);
	// The section loader awaits the read, so the heading means the data is in.
	await expect(page.getByRole('heading', { name: 'Workspaces', exact: true }).first()).toBeVisible();
	// A refused read (the old `lead` floor: 403) renders this notice, not rows.
	await expect(page.locator('[data-load-error]')).toHaveCount(0);
}

const filedRow = (page: Page) => page.getByRole('button', { name: new RegExp(workspace.name) });

async function filedState(): Promise<{ workspaces: { id: string }[]; links: number }> {
	const workspaces = ok(
		await dojo.db.from('workspaces').select('id').eq('tenant_id', tenantId).eq('slug', workspace.slug),
		'read filed workspaces'
	) as { id: string }[];
	const links = ok(
		await dojo.db.from('repositories_in_workspaces').select('workspace_id').eq('tenant_id', tenantId),
		'read links'
	) as unknown[];
	return { workspaces, links: links.length };
}

test.describe.serial('the sync files a repository under its workspace (#299)', () => {
	let firstWorkspaceId: string;

	test.beforeAll(async () => {
		dojo = localDojo();
		await removeFixtures(dojo, PREFIX);
		rho = await createUser(dojo, `${PREFIX}-rho-${run}@local.test`, 'rho');
		lex = await createUser(dojo, `${PREFIX}-lex-${run}@local.test`, 'lex');
		const tenant = ok(
			await dojo.db
				.from('tenants')
				.insert({ key: `organization/${org}`, origin: 'organization', slug: org, name: org, dojo_url: DOJO_URL })
				.select('id')
				.single(),
			'create tenant'
		) as { id: string };
		tenantId = tenant.id;
		ok(
			await dojo.db.from('tenant_connections').insert({
				tenant_id: tenantId,
				provider: 'github',
				external_slug: org,
				connected_by: rho.principalId
			}),
			'connect the forge org'
		);
		ok(
			await dojo.db.from('memberships').insert([
				{ tenant_id: tenantId, user_id: rho.principalId, role: 'contributor', kind: 'employer', authenticated_via: 'github_oauth' },
				{ tenant_id: tenantId, user_id: lex.principalId, role: 'lead', kind: 'employer', authenticated_via: 'github_oauth' }
			]),
			'add members'
		);
	});

	test.afterAll(async () => {
		if (dojo) await removeFixtures(dojo, PREFIX);
	});

	test('cycle 1 files the connected repository and reports the other unmapped', async () => {
		const reg = await syncCycle(rho);

		expect(reg.mapped.map((m) => m.repo_key)).toEqual([filedKey]);
		expect(reg.mapped[0].workspace_error).toBeUndefined();
		expect(reg.mapped[0].workspace_id).toBeTruthy();
		firstWorkspaceId = reg.mapped[0].workspace_id!;
		expect(reg.unmapped).toEqual([{ repo_key: unmappedKey, reason: 'no_connection' }]);

		const state = await filedState();
		expect(state.workspaces.map((w) => w.id)).toEqual([firstWorkspaceId]);
		expect(state.links).toBe(1);

		// rho's forge role on the repository (the mirror the daemon refreshes from
		// the forge, seeded here: no forge is reachable from a local run).
		ok(
			await dojo.db.from('repository_roles').insert({
				repository_id: reg.mapped[0].repo_id,
				principal_id: rho.principalId,
				forge_permission: 'write',
				forge_role: 'contributor'
			}),
			'grant rho a role on the repository'
		);
	});

	test('a contributor with a role on the repository sees the workspace once', async ({ page }) => {
		await signIn(page, rho);
		await openWorkspaces(page);
		await expect(filedRow(page)).toHaveCount(1);
	});

	test('cycle 2 changes nothing: same workspace, one link, still listed once', async ({ page }) => {
		const reg = await syncCycle(rho);
		expect(reg.mapped.map((m) => m.workspace_id)).toEqual([firstWorkspaceId]);
		expect(reg.unmapped).toEqual([{ repo_key: unmappedKey, reason: 'no_connection' }]);

		const state = await filedState();
		expect(state.workspaces.map((w) => w.id)).toEqual([firstWorkspaceId]);
		expect(state.links).toBe(1);

		await signIn(page, rho);
		await openWorkspaces(page);
		await expect(filedRow(page)).toHaveCount(1);
	});

	test('a lead with no role on the repository does not see the workspace', async ({ page }) => {
		await signIn(page, lex);
		await openWorkspaces(page);
		await expect(page.getByText('No workspaces yet.')).toBeVisible();
		await expect(filedRow(page)).toHaveCount(0);
	});

	test('the unmapped repository files nothing anywhere', async () => {
		const repos = ok(
			await dojo.db.from('repositories').select('id').eq('repo_key', unmappedKey),
			'read unmapped repository'
		) as unknown[];
		expect(repos).toHaveLength(0);
		const workspaces = ok(
			await dojo.db.from('workspaces').select('tenant_id').eq('slug', workspace.slug),
			'read workspaces by slug'
		) as { tenant_id: string }[];
		expect(workspaces.map((w) => w.tenant_id)).toEqual([tenantId]);
	});
});
