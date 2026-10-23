// Magic-link membership invites (F3b). An admin issues an invite for their
// tenant (email + role + kind); the invitee, authenticated as that email,
// redeems it once and gets a membership at the invited role. Security gates on
// accept are ALL fail-closed: the token must exist, be unexpired, be unused, AND
// the authenticated caller's email must match the invite (Supabase magic-link has
// proven ownership — the token alone is never sufficient). The token is an
// unguessable single-use bearer (crypto.randomUUID). Reuses `addMember` for the
// membership write (its own duplicate guard).
import {
	AdminError,
	addMember,
	isMemberRole,
	MEMBERSHIP_KINDS,
	type DojoClient,
	type NewMemberInput
} from './admin-data';
import { looksLikeEmail } from '../admin-view';

export { AdminError };
export type { DojoClient };

/** How long an invite is valid — 7 days from issue. */
const INVITE_TTL_MS = 7 * 24 * 60 * 60 * 1000;

/** A validated `POST …/invites` body. */
export interface NewInviteInput {
	email: string;
	role: string;
	kind: string;
	/** `member` makes a membership at `role`; `viewer` (#307 D8) makes the person a
	 *  dōjō viewer — not a member — whom an admin then grants selected workspaces. */
	access: InviteAccess;
}

export type InviteAccess = 'member' | 'viewer';
const INVITE_ACCESS: readonly InviteAccess[] = ['member', 'viewer'];

/** Validate a create-invite body into a {@link NewInviteInput}, or throw
 *  AdminError(400). `email` required + shaped; `role` a member_role (default
 *  contributor); `kind` a membership_kind. */
export function parseNewInvite(body: Record<string, unknown>): NewInviteInput {
	const email = typeof body.email === 'string' ? body.email.trim().toLowerCase() : '';
	if (!email || !looksLikeEmail(email)) throw new AdminError(400, 'a valid email is required');
	const role = isMemberRole(body.role) ? body.role : 'contributor';
	const kind = typeof body.kind === 'string' ? body.kind : '';
	if (!(MEMBERSHIP_KINDS as readonly string[]).includes(kind)) {
		throw new AdminError(400, 'kind must be employer, client, community, or personal');
	}
	const access = body.access === undefined ? 'member' : body.access;
	if (!(INVITE_ACCESS as readonly unknown[]).includes(access)) {
		throw new AdminError(400, 'access must be member or viewer');
	}
	return { email, role, kind, access: access as InviteAccess };
}

/**
 * Issue an invite for a tenant (admin-only — the route enforces the floor). The
 * token is an unguessable single-use bearer carried in the accept link. Returns
 * the row the admin needs to share the link (never emailed here — delivery is a
 * follow-on). Fails closed.
 */
export async function createInvite(
	db: DojoClient,
	tenantId: string,
	invitedBy: string,
	input: NewInviteInput,
	nowMs: number
): Promise<{ id: string; token: string; email: string; role: string; expires_at: string }> {
	const token = crypto.randomUUID();
	const tokenHash = await hashInviteToken(token);
	const expiresAt = new Date(nowMs + INVITE_TTL_MS).toISOString();
	const { data, error } = await db
		.from('invites')
		.insert({
			tenant_id: tenantId,
			email: input.email,
			role: input.role,
			kind: input.kind,
			access: input.access,
			// Only a hash of the bearer is stored (#311); the admin is handed the token.
			token_hash: tokenHash,
			invited_by: invitedBy,
			expires_at: expiresAt
		})
		// Not `token_hash`: no session reads it back (the read grant leaves it out,
		// #236). The admin is handed the token minted above.
		.select('id, email, role, expires_at')
		.single();
	if (error) throw new AdminError(500, error.message);
	return { ...(data as { id: string; email: string; role: string; expires_at: string }), token };
}

/** One invite as the accept path reads it (before validating). */
interface InviteRow {
	id: string;
	tenant_id: string;
	email: string;
	role: string;
	kind: string;
	access: InviteAccess;
	invited_by: string;
	expires_at: string;
	accepted_at: string | null;
}

async function loadInvite(db: DojoClient, token: string): Promise<InviteRow> {
	if (!token) throw new AdminError(400, 'token is required');
	const { data, error } = await db
		.from('invites')
		.select('id, tenant_id, email, role, kind, access, invited_by, expires_at, accepted_at')
		.eq('token_hash', await hashInviteToken(token))
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	if (!data) throw new AdminError(404, 'invalid or expired invite');
	return data as InviteRow;
}

/**
 * Redeem an invite. ALL gates fail closed and NONE create a membership unless
 * every one passes:
 *  - token exists (else 404),
 *  - not already accepted (else 409 — single-use),
 *  - not expired (else 410),
 *  - the authenticated caller's email matches the invite (else 403 — the real
 *    authorization gate; a leaked token can't be redeemed by anyone else).
 * On success, provisions the membership at the invited role (reusing addMember)
 * — or, for a viewer invite (#307 D8), records a dōjō viewer, never a membership —
 * and stamps `accepted_at` (single-use). Returns the tenant and what was joined.
 */
export async function acceptInvite(
	db: DojoClient,
	userId: string,
	callerEmail: string | null,
	token: string,
	nowMs: number
): Promise<{ tenant_id: string; role: string | null; access: InviteAccess }> {
	const invite = await loadInvite(db, token);
	if (invite.accepted_at) throw new AdminError(409, 'invite already used');
	if (Date.parse(invite.expires_at) < nowMs) throw new AdminError(410, 'invite expired');
	// THE gate: only the invited email can redeem, and Supabase has proven the
	// caller owns it. A missing/mismatched caller email is rejected.
	if (!callerEmail || callerEmail.trim().toLowerCase() !== invite.email.trim().toLowerCase()) {
		throw new AdminError(403, 'this invite is for a different email');
	}
	if (invite.access === 'viewer') {
		// A viewer is in the dōjō but is not a member: no member surface, no
		// repository — an admin grants them the view of selected workspaces.
		const { error } = await db.rpc('add_dojo_viewer', {
			p_tenant: invite.tenant_id,
			p_principal: userId,
			p_invited_by: invite.invited_by,
			// The email they were invited at, and just proved by signing in — what
			// names a viewer who has no forge identity (#307 D8).
			p_email: invite.email.trim().toLowerCase()
		});
		if (error) throw new AdminError(500, error.message);
	} else {
		// Provision the membership at the invited role (addMember guards a duplicate).
		await addMember(db, invite.tenant_id, {
			user_id: userId,
			kind: invite.kind,
			authenticated_via: 'sso',
			// The DB stores a valid `dojo.member_role`; narrow the string to the union.
			role: invite.role as NewMemberInput['role']
		});
	}
	// Single-use: stamp accepted (guarded so a race can't re-stamp), and drop the
	// token's hash — an accepted invite needs no bearer (#311).
	const { error } = await db
		.from('invites')
		.update({ accepted_at: new Date(nowMs).toISOString(), token_hash: null })
		.eq('id', invite.id)
		.is('accepted_at', null);
	if (error) throw new AdminError(500, error.message);
	return invite.access === 'viewer'
		? { tenant_id: invite.tenant_id, role: null, access: 'viewer' }
		: { tenant_id: invite.tenant_id, role: invite.role, access: 'member' };
}

/** The hex SHA-256 of an invite token — what the row stores and the accept path
 *  looks up (#311). The token itself is a bearer and is never stored. */
export async function hashInviteToken(token: string): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(token));
	return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}


