// Provision at the one moment the forge token exists.
//
// kavach calls this on the server from its session-sync hook, with the INCOMING
// provider session — the payload the browser POSTs to `/auth/session`. That is
// the ONLY place `provider_token` is reachable server-side: the session cookie
// keeps just access_token/refresh_token (deliberately — a GitHub token has no
// business living in a cookie for the session's lifetime), so by the very next
// request it is gone.
//
// This is what §II.7 means by "provider_token exists only immediately after the
// OAuth exchange, therefore provisioning MUST run in the sign-in callback". The
// dōjō has no callback of its own to hook (kavach owns it — §VIII.3), and this
// hook is that seam.
//
// The dōjō reads the org list itself from the token; it never accepts a list of
// orgs from a caller, which would be the service trusting a client about its own
// entitlements (§II.5).
import { dojoDb } from './dojo-supabase';
import { resolvePrincipalId } from './principal-resolve';
import { provisionWithToken } from './provisioning';
import { githubIdentityId } from './github-identity';
import { microsoftIdentity } from './azure-signin';
import { AdminError } from './admin-data';

/** The provider session as the browser sends it, narrowed to what we read. */
interface IncomingSession {
	access_token?: unknown;
	provider_token?: string | null;
	user?: { id?: unknown; email?: unknown } | null;
}

/** Auth events worth provisioning on. `TOKEN_REFRESHED` fires on every silent
 *  renewal and carries no provider_token, so acting on it would be a GitHub
 *  round trip that can only ever answer `no_forge_token`. */
const PROVISIONING_EVENTS = new Set(['SIGNED_IN', 'INITIAL_SESSION', 'USER_UPDATED']);

/**
 * Establish the caller's personal dōjō, plus a tenant per forge org their token
 * proves. Idempotent, so the repeat calls kavach makes (SIGNED_IN and
 * INITIAL_SESSION both fire) converge instead of duplicating.
 *
 * Errors are NOT swallowed here. kavach already isolates a throwing hook from
 * the sign-in — the session survives and the failure is logged — and swallowing
 * it a second time would hide it from both sides.
 */
export async function provisionOnSignIn(
	session: IncomingSession | null | undefined,
	event: string
): Promise<void> {
	if (!session || !PROVISIONING_EVENTS.has(event)) return;

	// Who this is, by Supabase's word for the access token — never the session's
	// own `user` claim — the GitHub account linked to that login, which is the
	// only account whose token provisioning will use (#308 R5a review), and the
	// Microsoft account, whose Entra object id provisions them from the stored
	// Azure DevOps member sets (#308 R5c).
	const accessToken = typeof session.access_token === 'string' ? session.access_token : '';
	if (!accessToken) return; // not a session we can attribute to anyone

	const db = dojoDb();
	const { data, error } = await db.auth.getUser(accessToken);
	if (error || !data?.user) throw new AdminError(401, 'Supabase would not vouch for this session');
	const principalId = await resolvePrincipalId(db, data.user.id);
	await provisionWithToken(db, principalId, session.provider_token ?? null, {
		email: data.user.email ?? null,
		githubId: githubIdentityId(data.user),
		microsoft: microsoftIdentity(data.user)
	});
}
