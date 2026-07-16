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

	// Magic link (email OTP) — for organizations not on GitHub. The email is
	// delivered by the local supabase stack's Inbucket during dev.
	providers: [{ mode: 'otp', name: 'magic', label: 'Email me a magic link' }],

	logging: { level: 'error', table: 'audit.logs' },

	// App routes the sentry guard needs to know about.
	routes: {
		auth: '/signin',
		logout: '/logout',
		session: '/auth/session',
		home: '/orgs'
	},

	// Route protection. Public: landing, sign-in and the org picker (reached
	// straight after auth). The guarded console group requires an authenticated
	// session — any signed-in role may open it; per-org role gating lives in the
	// R9–R11 console screens, not in this shell.
	rules: [
		{ path: '/', public: true },
		{ path: '/signin', public: true },
		// The machine/token API (/v1/*) self-authenticates in-handler (device-token
		// or Supabase-JWT plane, see dojo-auth.ts) — it must bypass kavach's web
		// session guard, or every daemon/API call 303-redirects to /signin.
		{ path: '/v1', public: true },
		{ path: '/orgs', roles: '*' },
		{ path: '/console', roles: '*' }
	]
};
