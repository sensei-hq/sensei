// POST /v1/you/github/sync — the console's explicit "sync from GitHub" button,
// and the third caller of §II.7.
//
// It is now literally the same operation as POST /v1/you/provision, so it
// re-exports that handler rather than keeping a second copy in step. The two
// paths differ only in who presses the button.
//
// What it used to be: `syncGithubMemberships`, which joined only tenants that
// ALREADY existed — "never invents a tenant" — so the first user in an org had
// nothing to join, and it silently returned `{ synced: false }` whenever
// `session.provider_token` had expired out of the session, which is every call
// after the OAuth exchange. That module is gone; this now provisions.
export { POST } from '../../provision/+server';
