// Sign in with Microsoft records the person's Microsoft Entra OBJECT ID (`oid`)
// as their Azure identity (#308 R5c, D45) — the id R5b's import and member read
// key on (Azure DevOps Graph `originId`, origin `aad`). Never `sub` (pairwise:
// a different value per application) and never email (D43).
//
// Where GoTrue puts it (supabase/auth, internal/api/provider/oidc.go,
// parseAzureIDToken): every ID-token claim except a fixed list (aud, iss, iat,
// nbf, exp, c_hash, at_hash, aio, nonce, rh, uti, jti, ver, sub, name,
// preferred_username) goes to `Metadata.CustomClaims`, serialised as
// `custom_claims` (provider.go `Claims`), and the identity's `identity_data` is
// that map (external.go), rewritten on every sign-in. So the oid is
// `identities[provider = azure].identity_data.custom_claims.oid`. The identity's
// own `id` is GoTrue's provider_id, the `sub`. `user_metadata` also receives the
// claims, but a person can rewrite their own user_metadata — it is never read.
import { describe, it, expect } from 'vitest';
import { microsoftIdentity, microsoftLinkStatus, NO_OBJECT_ID, ALREADY_LINKED, NOT_RECORDED, EMAIL_NOT_VERIFIED_BY_MICROSOFT } from './azure-signin';
import { fakeDojoDb } from './fake-dojo-db';

const OID = '00000000-0000-4000-8000-0000000000a1';

const user = (identities: unknown[], extra: Record<string, unknown> = {}) => ({ id: 'auth-alice', identities, ...extra });
const azure = (identity_data: unknown, id = 'pairwise-sub-xyz') => ({ provider: 'azure', id, identity_data });

describe('microsoftIdentity', () => {
	it('is the object id from the Microsoft identity GoTrue recorded, with the name', () => {
		const u = user([
			{ provider: 'github', id: '111' },
			azure({ sub: 'pairwise-sub-xyz', full_name: 'Alice Example', custom_claims: { oid: OID, tid: 'tenant-1', xms_edov: true } })
		]);
		expect(microsoftIdentity(u)).toEqual({ state: 'ok', objectId: OID, name: 'Alice Example' });
	});

	it('is none when the login has no Microsoft identity', () => {
		for (const u of [null, undefined, {}, user([]), user([{ provider: 'github', id: '111' }])]) {
			expect(microsoftIdentity(u)).toEqual({ state: 'none' });
		}
	});

	it('is unlinkable — never sub, never email — when GoTrue recorded no object id', () => {
		for (const data of [
			{ sub: 'pairwise-sub-xyz', email: 'alice@example.com' },
			{ sub: 'pairwise-sub-xyz', custom_claims: {} },
			{ custom_claims: { oid: '' } },
			{ custom_claims: { oid: 42 } },
			{ custom_claims: { oid: 'not-a-guid' } },
			null
		]) {
			const out = microsoftIdentity(user([azure(data)]));
			expect(out.state).toBe('unlinkable');
			expect(JSON.stringify(out)).not.toContain('pairwise-sub-xyz');
			expect(JSON.stringify(out)).not.toContain('alice@example.com');
		}
	});

	// The nOAuth exposure (#308 R5c security review): Supabase links a new
	// identity to an existing login by VERIFIED email, and without `xms_edov`
	// GoTrue assumes a Microsoft email is verified. The dōjō cannot stop that link
	// (the app registration's xms_edov claim does), but it never records or
	// provisions from a Microsoft account whose email Microsoft did not confirm.
	it('is unlinkable unless Microsoft confirmed the email (xms_edov), whatever the object id', () => {
		for (const edov of [undefined, false, 0, '0', 'false', '', null, 'yes']) {
			const out = microsoftIdentity(user([azure({ custom_claims: { oid: OID, ...(edov === undefined ? {} : { xms_edov: edov }) } })]));
			expect(out).toEqual({ state: 'unlinkable', reason: EMAIL_NOT_VERIFIED_BY_MICROSOFT });
		}
	});

	it('accepts every way Microsoft writes a confirmed xms_edov', () => {
		for (const edov of [true, '1', 1, 'true']) {
			expect(microsoftIdentity(user([azure({ custom_claims: { oid: OID, xms_edov: edov } })]))).toMatchObject({ state: 'ok', objectId: OID });
		}
	});

	it('never reads the object id from user_metadata, which the person can rewrite', () => {
		const u = user([azure({ sub: 'pairwise-sub-xyz' })], { user_metadata: { custom_claims: { oid: OID } } });
		expect(microsoftIdentity(u).state).toBe('unlinkable');
	});

	it('is unlinkable when more than one Microsoft account is linked — which one would be guessing', () => {
		const other = '00000000-0000-4000-8000-0000000000a2';
		const u = user([azure({ custom_claims: { oid: OID, xms_edov: true } }, 's1'), azure({ custom_claims: { oid: other, xms_edov: true } }, 's2')]);
		expect(microsoftIdentity(u)).toMatchObject({ state: 'unlinkable' });
	});

	it('lowercases the object id, as the R5b member read and import key it (forge-azure entraObjectId)', () => {
		const upper = '00000000-0000-4000-8000-0000000000A1';
		expect(microsoftIdentity(user([azure({ custom_claims: { oid: upper, xms_edov: true } })]))).toMatchObject({ objectId: OID });
	});
});

// What My dōjōs says about the login's Microsoft account (#308 R5c): computed
// from Supabase's identity and the dōjō's own record, never stored — so a failed
// link is visible on the next page load, whatever the sign-in hook logged.
describe('microsoftLinkStatus', () => {
	const ME = 'p-me';
	const ok = { state: 'ok' as const, objectId: OID, name: null };
	const db = (rows: Record<string, unknown>[] = [], error?: { message: string }) =>
		fakeDojoDb({ identities: { rows, ...(error ? { error } : {}) } });

	it('is none for a login with no Microsoft account, asking nothing', async () => {
		const d = db();
		expect(await microsoftLinkStatus(d as never, ME, { state: 'none' })).toEqual({ state: 'none' });
		expect(d.tables.identities.reads ?? 0).toBe(0);
	});

	it('is linked when the object id is recorded as this person`s azure_devops identity', async () => {
		const d = db([{ principal_id: ME, provider: 'azure_devops', subject: OID }]);
		expect(await microsoftLinkStatus(d as never, ME, ok)).toEqual({ state: 'linked' });
	});

	it('is not linked, with the reason, when Microsoft sent no object id', async () => {
		expect(await microsoftLinkStatus(db() as never, ME, { state: 'unlinkable', reason: NO_OBJECT_ID })).toEqual({
			state: 'not_linked',
			reason: NO_OBJECT_ID
		});
	});

	it('is not linked when another person holds the Microsoft account', async () => {
		const d = db([{ principal_id: 'p-other', provider: 'azure_devops', subject: OID }]);
		expect(await microsoftLinkStatus(d as never, ME, ok)).toEqual({ state: 'not_linked', reason: ALREADY_LINKED });
	});

	it('is not linked when the sign-in never recorded it', async () => {
		expect(await microsoftLinkStatus(db() as never, ME, ok)).toEqual({ state: 'not_linked', reason: NOT_RECORDED });
	});

	it('a failed read throws — never "not linked"', async () => {
		await expect(microsoftLinkStatus(db([], { message: 'boom' }) as never, ME, ok)).rejects.toThrow('boom');
	});
});
