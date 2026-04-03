export default {
  adapter: 'supabase',
  env: { url: 'PUBLIC_SUPABASE_URL', anonKey: 'PUBLIC_SUPABASE_ANON_KEY' },
  app: {
    home: '/',
    login: '/auth',
    logout: '/logout',
    session: '/auth/session',
    data: '/data',
  },
  // Phase 1: all routes public. Phase 9 adds role-based protection.
  rules: [
    { path: '/', public: true },
    { path: '/data', public: true },
    { path: '/mockups', public: true },
  ],
  providers: [
    { mode: 'password', name: 'email', label: 'Sign in with Email' },
  ],
}
