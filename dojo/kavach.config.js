// Dōjō console — kavach configuration.
//
// Mirrors sites/demo/kavach.config.js (the reference setup), reduced to the
// supabase adapter only. The @kavach/vite plugin reads this file at build time
// and generates the virtual $kavach/* modules ($kavach/auth, $kavach/providers,
// …). `$kavach/auth` wires @kavach/adapter-supabase + the supabase client and
// creates the kavach instance whose `.handle` (from @kavach/sentry) guards
// routes in src/hooks.server.ts.
//
// Magic-link (OTP) is the primary sign-in for orgs not on GitHub — see the
// DojoSignIn mockup. GitHub OAuth is planned but out of scope for R6 (the
// supabase local stack from R5 provides the OTP/magic-link path via Inbucket).

export default {
	adapter: 'supabase',
	env: { url: 'PUBLIC_SUPABASE_URL', anonKey: 'PUBLIC_SUPABASE_ANON_KEY' },

	// Sign-in providers. kavach's generic signIn handles each by name from this
	// config (one flow for oauth / magic-link / password — no per-provider code):
	//  - `magic` (email OTP), delivered by the local supabase Inbucket in dev.
	//  - `github` (OAuth). `scopes` are requested at sign-on so the session's
	//    GitHub token can read the user's org memberships — `read:org` covers
	//    PRIVATE orgs — for the F3c GitHub-org auto-join (GET /user/orgs);
	//    `user:email` keeps the email Supabase resolves the profile from.
	//
	//    `repo` is what lets `refreshForgeVisibility` SEE a private repository.
	//    GitHub answers 404 (not 403) for a repo a token cannot reach, so without
	//    it every private repo stays permanently uncaptured and — failing closed —
	//    never syncs. That is the exact case the authority/mandate model exists
	//    for, so capture without it works only where it is not needed.
	//
	//    It is a BIG grant for a small answer: classic `repo` is full read AND
	//    WRITE over all private code, and there is no metadata-only classic scope.
	//    The right-sized permission (`metadata: read`) exists only for a GitHub
	//    App, which is the migration this should become — tracked in
	//    docs/backlog.md.
	//
	//    Where the token then LIVES differs by path, and the difference matters
	//    now that the grant is this broad:
	//      web  — `provider_token` exists only in the payload handed to
	//             `onSessionSync`; the session cookie keeps access/refresh only,
	//             so it is unreachable from every later request (hooks.server.ts).
	//      CLI  — `/v1/auth/cli/token` returns the session byte-for-byte and the
	//             daemon PERSISTS it, in the OS keychain rather than Postgres
	//             (`dojo_client/session.rs`). So on a developer machine this is a
	//             stored, refreshable, full-private-code credential — which is the
	//             strongest argument for the GitHub App migration above.
	providers: [
		{ mode: 'oauth', name: 'github', label: 'Continue with GitHub', scopes: ['read:org', 'user:email', 'repo'] },
		{ mode: 'otp', name: 'magic', label: 'Email me a magic link' },
		// Email + password. `name: 'email'` is the field key AuthProvider passes to
		// signIn ({ email, password }) → the adapter's signInWithPassword.
		{ mode: 'password', name: 'email', label: 'Sign in with password' }
	],

	logging: { level: 'error', table: 'audit.logs' },

	// App routes the sentry guard needs to know about.
	routes: {
		auth: '/signin',
		logout: '/logout',
		session: '/auth/session',
		// Post-auth landing = the dojo2 personal home (`/you`). It works solo for a
		// membership-less user (DJ1 — no fabricated tenant) and is populated for
		// members; entering a specific org is a second step (/orgs → Enter →
		// /org/{slug}). The org picker (/orgs) is still reachable as "manage".
		home: '/you'
	},

	// Route protection. Public: landing, sign-in and the org picker (reached
	// straight after auth). The guarded console group requires an authenticated
	// session — any signed-in role may open it; per-org role gating lives in the
	// R9–R11 console screens, not in this shell.
	rules: [
		{ path: '/', public: true },
		{ path: '/signin', public: true },
		// The served logout page (src/routes/logout/+page.svelte) runs the client
		// signOut() then redirects to /signin. Public so the guard lets it render
		// even when the session is already partly cleared; the pinned kavach does
		// not serve `routes.logout` itself, so a bare /logout would otherwise 404.
		{ path: '/logout', public: true },
		// Build-time version stamp (GET /version → src/lib/version.ts). Must be
		// reachable unauthenticated so `curl .../version` confirms the live deploy.
		// Unmatched paths fall through to the sentry guard's 401→/signin redirect,
		// so an explicit public rule is required to exempt it.
		{ path: '/version', public: true },
		// The machine/token API (/v1/*) self-authenticates in-handler (device-token
		// or Supabase-JWT plane, see dojo-auth.ts) — it must bypass kavach's web
		// session guard, or every daemon/API call 303-redirects to /signin.
		{ path: '/v1', public: true },
		{ path: '/orgs', roles: '*' },
		{ path: '/console', roles: '*' },
		// The dojo2 shell (rebuild in progress): the personal zone (/you) and the
		// role-scoped org zone (/org/{slug}). Same guard as the console group — any
		// signed-in role may open it; per-org role gating lives in the nav + loads.
		{ path: '/you', roles: '*' },
		{ path: '/org', roles: '*' }
	]
};
