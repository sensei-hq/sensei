export default {
  adapter: 'supabase',
  env: { url: 'PUBLIC_SUPABASE_URL', anonKey: 'PUBLIC_SUPABASE_ANON_KEY' },
  app: {
    home: '/',
    login: '/login',
    logout: '/logout',
    session: '/auth/session',
    data: '/data',
  },
  // Phase 1: all routes public. Phase 9 adds role-based protection.
  rules: [
    { path: '/', public: true },
    { path: '/data', public: true },
  ],
  providers: [
    // Phase 1: no providers configured. Phase 9 adds GitHub OAuth.
  ],
}
