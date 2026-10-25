import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import ScrWorkspaces from './ScrWorkspaces.svelte';
import ScrTriage from './ScrTriage.svelte';
import ScrApprovals from './ScrApprovals.svelte';
import ScrKnowledge from './ScrKnowledge.svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import ScrIdentity from './ScrIdentity.svelte';
import ScrHealth from './ScrHealth.svelte';
import ScrBilling from './ScrBilling.svelte';
import { toKitCandidateDetail } from '$lib/triage-map';
import { toKitIdentity, toKitHealth } from '$lib/admin-map';
import { noBilling } from '$lib/billing-map';

// #335: an org screen whose read failed shows the failure, in place of the empty
// state it would otherwise show — a failed read is never an empty jurisdiction.
const failed = 'boom';
const SCREENS = [
	['ScrWorkspaces', ScrWorkspaces, { workspaces: [] }],
	['ScrTriage', ScrTriage, { orgName: 'o', groups: [], detail: toKitCandidateDetail(undefined) }],
	['ScrApprovals', ScrApprovals, { orgName: 'o', approvals: [] }],
	['ScrKnowledge', ScrKnowledge, { orgName: 'o', knowledge: { prunePolicy: '', active: [], pending: [], catalog: [] } }],
	['ScrRoleSurfaces', ScrRoleSurfaces, { orgName: 'o' }],
	['ScrIdentity', ScrIdentity, { orgName: 'o', identity: toKitIdentity([]) }],
	['ScrHealth', ScrHealth, { orgName: 'o', health: toKitHealth({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 0 }) }],
	['ScrBilling', ScrBilling, { orgName: 'o', billing: noBilling() }]
] as const;

describe('org screens show a failed read (#335)', () => {
	afterEach(cleanup);

	for (const [name, Screen, props] of SCREENS) {
		it(`${name} shows the error`, () => {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const { getByRole } = render(Screen as any, { props: { ...props, error: failed } });
			expect(getByRole('alert').textContent).toContain(failed);
		});
	}

	it('the org page passes every loader error to its screen', () => {
		const dir = 'src/routes/(app)/org/[slug]/[section]';
		const loader = readFileSync(`${dir}/+page.ts`, 'utf8');
		const page = readFileSync(`${dir}/+page.svelte`, 'utf8');
		const fields = [...new Set([...loader.matchAll(/^\t\t(\w+Error):/gm)].map((m) => m[1]))];
		expect(fields.length).toBeGreaterThan(5);
		expect(fields.filter((f) => !page.includes(`data.${f}`))).toEqual([]);
	});
});
