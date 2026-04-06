export default {
	adapter: 'supabase',
	providers: [
		{
			mode: 'password',
			name: 'email',
			label: 'Sign in with Email'
		}
	],
	cachedLogins: false,
	env: {
		url: 'PUBLIC_SUPABASE_URL',
		anonKey: 'PUBLIC_SUPABASE_ANON_KEY'
	},
	routes: {
		auth: '/auth',
		data: '/data',
		rpc: '/rpc',
		logout: '/logout',
		home: '/home',
		session: '/auth/session'
	},
	rules: [
		{
			path: '/',
			public: true
		},
		{
			path: '/data',
			public: true
		},
		{
			path: '/mockups',
			public: true
		},
		{
			path: '/api/mockups',
			public: true
		},
		{
			path: '/platform',
			roles: ['platform_admin']
		}
	]
}
