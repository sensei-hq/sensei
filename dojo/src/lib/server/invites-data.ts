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

export { AdminError };
export type { DojoClient };

/** How long an invite is valid — 7 days from issue. */
const INVITE_TTL_MS = 7 * 24 * 60 * 60 * 1000;

/** A validated `POST …/invites` body. */
export interface NewInviteInput {
	email: string;
	role: string;
	kind: string;
}

/** A minimal, conservative email check — a single `@` with non-empty local and
 *  domain parts. The real proof-of-ownership is Supabase auth at accept time;
 *  this only rejects obvious garbage before issuing. */
function looksLikeEmail(s: string): boolean {
	const at = s.indexOf('@');
	return at > 0 && at < s.length - 1 && !s.includes(' ');
}

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
	return { email, role, kind };
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
	const expiresAt = new Date(nowMs + INVITE_TTL_MS).toISOString();
	const { data, error } = await db
		.from('invites')
		.insert({
			tenant_id: tenantId,
			email: input.email,
			role: input.role,
			kind: input.kind,
			token,
			invited_by: invitedBy,
			expires_at: expiresAt
		})
		.select('id, token, email, role, expires_at')
		.single();
	if (error) throw new AdminError(500, error.message);
	return data as { id: string; token: string; email: string; role: string; expires_at: string };
}

/** One invite as the accept path reads it (before validating). */
interface InviteRow {
	id: string;
	tenant_id: string;
	email: string;
	role: string;
	kind: string;
	expires_at: string;
	accepted_at: string | null;
}

async function loadInvite(db: DojoClient, token: string): Promise<InviteRow> {
	if (!token) throw new AdminError(400, 'token is required');
	const { data, error } = await db
		.from('invites')
		.select('id, tenant_id, email, role, kind, expires_at, accepted_at')
		.eq('token', token)
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
 * and stamps `accepted_at` (single-use). Returns the tenant + role joined.
 */
export async function acceptInvite(
	db: DojoClient,
	userId: string,
	callerEmail: string | null,
	token: string,
	nowMs: number
): Promise<{ tenant_id: string; role: string }> {
	const invite = await loadInvite(db, token);
	if (invite.accepted_at) throw new AdminError(409, 'invite already used');
	if (Date.parse(invite.expires_at) < nowMs) throw new AdminError(410, 'invite expired');
	// THE gate: only the invited email can redeem, and Supabase has proven the
	// caller owns it. A missing/mismatched caller email is rejected.
	if (!callerEmail || callerEmail.trim().toLowerCase() !== invite.email.trim().toLowerCase()) {
		throw new AdminError(403, 'this invite is for a different email');
	}
	// Provision the membership at the invited role (addMember guards a duplicate).
	await addMember(db, invite.tenant_id, {
		user_id: userId,
		kind: invite.kind,
		authenticated_via: 'sso',
		// The DB stores a valid `dojo.member_role`; narrow the string to the union.
		role: invite.role as NewMemberInput['role']
	});
	// Single-use: stamp accepted (guarded so a race can't re-stamp).
	const { error } = await db
		.from('invites')
		.update({ accepted_at: new Date(nowMs).toISOString() })
		.eq('id', invite.id)
		.is('accepted_at', null);
	if (error) throw new AdminError(500, error.message);
	return { tenant_id: invite.tenant_id, role: invite.role };
}
