// Azure DevOps on Admin → Integrations and in the personal dōjō (#308 R5b): the
// words the screen says about a connection, an import and the scopes a token
// needs. Pure.
import { describe, it, expect } from 'vitest';
import { AZURE_REQUIRED_SCOPES, connectNotice, importLine, syncLine, tokenLine, writeScopesLine, type AzureConnectionView, microsoftNotice } from './azure-view';

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
		expect(AZURE_REQUIRED_SCOPES.every((s) => s.label.endsWith('(read)'))).toBe(true);
	});
});

describe('tokenLine', () => {
	it('says whether the stored token works, needs replacing, or is missing — and its expiry when known', () => {
		expect(tokenLine(conn())).toBe('Connected through Microsoft Entra');
		expect(tokenLine(conn({ tokenExpiresAt: '2026-12-31T00:00:00Z' }))).toBe('Connected through Microsoft Entra · expires 2026-12-31');
		expect(tokenLine(conn({ token: 'needs_new_token' }))).toBe('Microsoft no longer accepts this connection — reconnect Azure DevOps');
		expect(tokenLine(conn({ token: 'none' }))).toBe('Not connected — connect Azure DevOps to sync');
	});
});

describe('importLine / syncLine', () => {
	const projects = { total: 3, mapped: 2, elsewhere: 1, complete: true };
	const members = { complete: true, listed: 12, ended: 0, joined: 0 };
	it('says where the organization went and how its projects filed', () => {
		expect(
			importLine({ dojo: { key: 'organization/example-org', created: true }, organization: { id: 'o', name: 'example-org' }, projects, members })
		).toBe('example-org is now the dōjō organization/example-org · 2 projects filed as workspaces · 1 already filed in another dōjō · 12 members read');
		expect(
			importLine({
				dojo: { key: 'organization/here', created: false },
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 1, mapped: 1, elsewhere: 0, complete: false },
				members: { complete: false, listed: 3, ended: 0, joined: 0 }
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
				members: { complete: true, listed: 4, ended: 1, joined: 0 }
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

	// A complete read also provisions the listed people who had already signed in
	// with Microsoft (#308 R5c): the line says how many joined.
	it('says who a complete read joined', () => {
		expect(
			syncLine({
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 0, mapped: 0, elsewhere: 0, complete: true },
				members: { complete: true, listed: 4, ended: 0, joined: 2 }
			})
		).toBe('example-org synced · no projects to file · 4 members read · 2 who had signed in now members');
		expect(
			syncLine({
				organization: { id: 'o', name: 'example-org' },
				projects: { total: 0, mapped: 0, elsewhere: 0, complete: true },
				members: { complete: true, listed: 4, ended: 1, joined: 1 }
			})
		).toBe('example-org synced · no projects to file · 4 members read · 1 who had signed in now a member · 1 who left the organization no longer a member');
	});
});

describe('connectNotice', () => {
	it("says what the connect came to, from the callback's query — and nothing for anything else", () => {
		expect(connectNotice(new URLSearchParams('azure=connected'))).toEqual({ ok: true, text: 'Connected to Azure DevOps — pick the organizations to import' });
		expect(connectNotice(new URLSearchParams('azure_error=denied'))).toEqual({ ok: false, text: 'The Microsoft sign-in was cancelled or consent was declined' });
		expect(connectNotice(new URLSearchParams('azure_error=not_configured'))?.text).toMatch(/not configured/);
		expect(connectNotice(new URLSearchParams('azure_error=expired'))?.text).toMatch(/expired/);
		expect(connectNotice(new URLSearchParams('azure_error=<script>'))).toBeNull();
		expect(connectNotice(new URLSearchParams(''))).toBeNull();
	});
});

describe('writeScopesLine', () => {
	it('names the extra scopes a consent carries, or nothing', () => {
		expect(writeScopesLine([])).toBeNull();
		expect(writeScopesLine(['vso.code_write', 'vso.project_manage'])).toBe(
			'This Microsoft consent also grants vso.code_write, vso.project_manage. The dōjō only reads — a Microsoft Entra admin can remove these from the app’s permissions.'
		);
	});
});

// My dōjōs says when a Microsoft sign-in could not be linked (#308 R5c): nothing
// was recorded and nobody provisioned, and the person should know why.
describe('microsoftNotice', () => {
	it('says nothing when there is no Microsoft account, or it is linked', () => {
		expect(microsoftNotice({ state: 'none' })).toBeNull();
		expect(microsoftNotice({ state: 'linked' })).toBeNull();
	});

	it('says the account could not be linked, and why', () => {
		expect(microsoftNotice({ state: 'not_linked', reason: 'more than one Microsoft account is linked to this login' })).toBe(
			'Couldn’t link your Microsoft account: more than one Microsoft account is linked to this login. Your Azure DevOps organizations’ dōjōs were not joined.'
		);
	});
});
