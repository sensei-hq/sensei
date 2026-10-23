// Who a Microsoft sign-in is, in Azure DevOps' terms (#308 R5c, D45).
//
// A Microsoft sign-in (Supabase Auth's Azure provider) records the person's
// Microsoft Entra OBJECT ID as their `azure_devops` identity: the id R5b's import
// and member read key on (Azure DevOps Graph `originId`, origin `aad`), so the
// stored member sets can provision them. Never `sub` — pairwise, a different
// value for every application — and never email (D43).
//
// WHERE GOTRUE KEEPS IT (supabase/auth, internal/api/provider/oidc.go,
// parseAzureIDToken): the verified ID token's claims, minus a fixed list (aud,
// iss, iat, nbf, exp, c_hash, at_hash, aio, nonce, rh, uti, jti, ver, sub, name,
// preferred_username), go to `Metadata.CustomClaims`, serialised as
// `custom_claims` (provider.go `Claims`); the identity's `identity_data` is that
// map (external.go), rewritten at every sign-in. So: `identities[provider =
// azure].identity_data.custom_claims.oid`. The identity's own `id` is GoTrue's
// provider_id — the `sub` — and is never used. `user_metadata` receives the same
// claims, but a person can rewrite their own user_metadata: never read.
//
// UNVERIFIED AGAINST A LIVE TENANT (no app registration yet): if a real sign-in
// carries no usable `oid` there, this answers `unlinkable` and the sign-in FAILS
// CLOSED — no identity, no provisioning, a logged reason, and "couldn't link your
// Microsoft account" on My dōjōs. The spec's "R5c as built" names the live check.

import { AdminError, type DojoClient } from './admin-data';
import type { MicrosoftLinkStatus } from '../azure-view';
import { entraObjectId } from './forge-azure';

/** What a login says about its Microsoft account. */
export type MicrosoftIdentity =
	| { state: 'none' }
	| { state: 'ok'; objectId: string; name: string | null }
	| { state: 'unlinkable'; reason: string };

/** Why a Microsoft identity could not be linked, as the person reads it. */
export const NO_OBJECT_ID = 'Microsoft did not send this account’s object id, so it cannot be matched to Azure DevOps';
export const SEVERAL_ACCOUNTS = 'more than one Microsoft account is linked to this login';
export const ALREADY_LINKED = 'that Microsoft account is already linked to another dōjō login';
export const NOT_RECORDED = 'the sign-in did not record it — sign out and sign in with Microsoft again';

function record(v: unknown): Record<string, unknown> | null {
	return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
}

/**
 * The Microsoft account linked to a login. `user` must come from Supabase Auth
 * (getUser, a synced or exchanged session), never from a request's own claim.
 */
export function microsoftIdentity(user: unknown): MicrosoftIdentity {
	const identities = record(user)?.identities;
	if (!Array.isArray(identities)) return { state: 'none' };
	const azure = identities.map(record).filter((i) => i?.provider === 'azure');
	if (azure.length === 0) return { state: 'none' };
	if (azure.length > 1) return { state: 'unlinkable', reason: SEVERAL_ACCOUNTS };

	const data = record(azure[0]?.identity_data);
	// Lowercased exactly as R5b keys a member read's originId (entraObjectId).
	const oid = entraObjectId(record(data?.custom_claims)?.oid);
	if (!oid) return { state: 'unlinkable', reason: NO_OBJECT_ID };
	const name = data?.full_name ?? data?.name;
	return { state: 'ok', objectId: oid, name: typeof name === 'string' && name.trim() ? name.trim() : null };
}

/**
 * Whether the login's Microsoft account is linked (#308 R5c): its object id
 * recorded as THIS person's azure_devops identity. Computed, never stored, so a
 * sign-in that failed closed shows on the next page load. `db` is the system
 * client: an identity another person holds is the conflict, and the caller
 * cannot read it. A failed read throws.
 */
export async function microsoftLinkStatus(
	db: DojoClient,
	principalId: string,
	identity: MicrosoftIdentity
): Promise<MicrosoftLinkStatus> {
	if (identity.state === 'none') return { state: 'none' };
	if (identity.state === 'unlinkable') return { state: 'not_linked', reason: identity.reason };
	const { data, error } = await db
		.from('identities')
		.select('principal_id')
		.eq('provider', 'azure_devops')
		.eq('subject', identity.objectId)
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	const holder = (data as { principal_id: string } | null)?.principal_id;
	if (!holder) return { state: 'not_linked', reason: NOT_RECORDED };
	return holder === principalId ? { state: 'linked' } : { state: 'not_linked', reason: ALREADY_LINKED };
}
