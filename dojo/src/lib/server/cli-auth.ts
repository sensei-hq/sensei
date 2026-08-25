// CLI sign-in — the URL arithmetic behind /v1/auth/cli/*.
//
// senseid is a PUBLIC OAuth client on the user's machine, so it signs in with
// PKCE and a loopback redirect. What it must NOT need is knowledge of where
// dōjō's auth actually lives: the daemon is configured with one setting, the
// dōjō site URL, and everything else is dōjō's business. That is what these
// endpoints are for — the daemon never sees a Supabase URL or key.
//
// It also keeps the provider allow-list to a single entry. If the daemon asked
// Supabase directly, every loopback port a daemon might bind would have to be
// allow-listed; instead Supabase only ever redirects to dōjō, and dōjō forwards
// to the loopback.
//
// Pure functions, so the flow's correctness is testable without a network, a
// browser, or a Supabase instance.

/** Where a daemon's loopback callback lives. Fixed path; only the port varies. */
const DAEMON_CALLBACK_PATH = '/api/auth/callback';

/** Scopes beyond the provider default, matching kavach.config.js's github provider. */
const EXTRA_SCOPES = ['read:org'];

/**
 * Ports a daemon may be forwarded to.
 *
 * The callback takes a port from the URL and redirects to it, so without a bound
 * it is an open redirect. Restricting to 127.0.0.1 (never a hostname, which
 * could resolve anywhere) and to the unprivileged range means the worst a
 * crafted link can do is bounce someone to a port on their own machine.
 */
export function isForwardablePort(port: unknown): port is number {
	const n = typeof port === 'string' ? Number(port) : port;
	return typeof n === 'number' && Number.isInteger(n) && n >= 1024 && n <= 65535;
}

/**
 * The provider authorize URL for a daemon sign-in.
 *
 * `redirect_to` points back at dōjō rather than the daemon: see the note above
 * on the allow-list. The daemon's port rides in the path so the callback knows
 * where to forward, and so a single `…/v1/auth/cli/callback/**` entry covers
 * every daemon.
 *
 * The code challenge is passed straight through. dōjō never sees the verifier —
 * it stays in the daemon's memory until the exchange, which is what stops a
 * captured redirect from being replayable by anyone, dōjō included.
 */
export function authorizeUrl(opts: {
	supabaseUrl: string;
	origin: string;
	port: number;
	challenge: string;
	provider?: string;
	login?: string | null;
}): string {
	const provider = opts.provider ?? 'github';
	const redirect = `${trimSlash(opts.origin)}/v1/auth/cli/callback/${opts.port}`;
	const u = new URL(`${trimSlash(opts.supabaseUrl)}/auth/v1/authorize`);
	u.searchParams.set('provider', provider);
	u.searchParams.set('redirect_to', redirect);
	u.searchParams.set('code_challenge', opts.challenge);
	u.searchParams.set('code_challenge_method', 'S256');
	// `scopes` APPENDS to the provider default, so pass only what is extra —
	// repeating user:email duplicates it on the consent screen the user reads.
	u.searchParams.set('scopes', EXTRA_SCOPES.join(' '));
	// Which account to suggest. Without it the browser's existing GitHub session
	// is reused, so connecting a SECOND identity quietly links the first one
	// again — a success as the wrong person, which is worse than a failure.
	if (opts.login) u.searchParams.set('login', opts.login);
	return u.toString();
}

/**
 * Where to send the browser once the provider comes back.
 *
 * Always 127.0.0.1, never a name from the request: a hostname could resolve
 * anywhere, and this URL is handed to the user's browser with an auth code on it.
 *
 * Every parameter is forwarded, not just `code`. An error arrives as
 * `?error=…&error_description=…` with no code at all, and dropping it would
 * leave the daemon waiting on a callback that already happened.
 */
export function daemonRedirect(port: number, params: URLSearchParams): string {
	const u = new URL(`http://127.0.0.1:${port}${DAEMON_CALLBACK_PATH}`);
	for (const [k, v] of params) u.searchParams.set(k, v);
	return u.toString();
}

function trimSlash(s: string): string {
	return s.replace(/\/+$/, '');
}
