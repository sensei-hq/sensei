export default {
  app: {
    home: '/',
    login: '/login',
    logout: '/logout',
    session: '/auth/session',
  },
  // Phase 1: no protected routes. Phase 9 adds role-based protection.
  roles: {},
  providers: [
    // Phase 1: no providers configured. Phase 9 adds GitHub OAuth.
  ],
}
