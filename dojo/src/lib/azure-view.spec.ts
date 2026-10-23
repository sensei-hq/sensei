// Azure DevOps on Admin → Integrations and in the personal dōjō (#308 R5b): the
// words the screen says about a connection, an import and the scopes a token
// needs. Pure.
import { describe, it, expect } from 'vitest';
import { AZURE_REQUIRED_SCOPES, importLine, syncLine, tokenLine, type AzureConnectionView } from './azure-view';

const conn = (over: Partial<AzureConnectionView> = {}): AzureConnectionView => ({
	id: 'org-1',
	name: 'example-org',
	verifiedAt: '2026-10-22T00:00:00Z',
	token: 'ok',
	tokenExpiresAt: null,
	...over
});

describe('AZURE_REQUIRED_SCOPES', () => {
	it('asks for read scopes only', () => {
		expect(AZURE_REQUIRED_SCOPES.every((s) => s.label.endsWith('(Read)'))).toBe(true);
	});
});

describe('tokenLine', () => {
	it('says whether the stored token works, needs replacing, or is missing — and its expiry when known', () => {
		expect(tokenLine(conn())).toBe('Token stored · expiry not reported by Azure DevOps');
		expect(tokenLine(conn({ tokenExpiresAt: '2026-12-31T00:00:00Z' }))).toBe('Token stored · expires 2026-12-31');
		expect(tokenLine(conn({ token: 'needs_new_token' }))).toBe('Azure DevOps no longer accepts the stored token — paste a new one');
		expect(tokenLine(conn({ token: 'none' }))).toBe('No token stored — paste one to sync');
	});
});

describe('importLine / syncLine', () => {
	const projects = { total: 3, mapped: 2, elsewhere: 1, complete: true };
	const members = { complete: true, listed: 12, ended: 0 };
	it('says where the organization went and how its projects filed', () => {
		expect(
			importLine({ dojo: { key: 'organization/example-org', created: true }, organization: { id: 'o', name: 'example-org' }, projects, members })
		).toBe('example-org is now the dōjō organization/example-org · 2 projects filed as workspaces · 1 already filed in another dōjō · 12 members read');
		expect(
			importLine({
				dojo: { key: 'organization/here', created: false },
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 1, mapped: 1, elsewhere: 0, complete: false },
				members: { complete: false, listed: 3, ended: 0 }
			})
		).toBe(
			'example-org is connected to organization/here · 1 project filed as a workspace · Azure DevOps could not list every project · the member list was incomplete, so nobody was ended'
		);
	});

	it('says what a sync filed', () => {
		expect(
			syncLine({
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 0, mapped: 0, elsewhere: 0, complete: true },
				members: { complete: true, listed: 4, ended: 1 }
			})
		).toBe('example-org synced · no projects to file · 4 members read · 1 who left the organization no longer a member');
		expect(
			syncLine({
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 0, mapped: 0, elsewhere: 0, complete: true },
				members: { error: 'Azure DevOps could not list the members (503); nobody was ended' }
			})
		).toBe('example-org synced · no projects to file · Azure DevOps could not list the members (503); nobody was ended');
	});
});
