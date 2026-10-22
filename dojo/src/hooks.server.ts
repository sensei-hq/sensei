import { kavach } from '$kavach/auth';
import type { Handle } from '@sveltejs/kit';
import { provisionOnSignIn } from '$lib/server/provision-on-signin';
import { captureForgeToken } from '$lib/server/forge-token-cookie';
import { serverSecret } from '$lib/server/dojo-supabase';
import kavachConfig from '../kavach.config.js';

// Mirror sites/demo/src/hooks.server.js: the kavach instance's `.handle`
// (built on @kavach/sentry) reads the session cookie into `event.locals.session`
// and enforces the `rules` from kavach.config.js — guarding `/console/*` (and
// `/orgs`) so an unauthenticated request is redirected to `/signin`.

// Provision on sign-in, at the one moment the forge token exists.
//
// kavach's session-sync hook is handed the INCOMING provider session — the
// payload the browser POSTs to `/auth/session`. The session COOKIE keeps only
// access_token/refresh_token, so `provider_token` is unreachable from every
// later request; this is the only seam where the dōjō can read the caller's
// GitHub orgs (spec §II.7, §VIII.3). Registered here rather than passed to
// `createKavach`, because the instance comes from the generated `$kavach/auth`
// module and takes no constructor options from us.
kavach.configure({ onSessionSync: provisionOnSignIn });

// The same moment is when a "Connect GitHub" re-authorization's token can be kept
// for the Integrations screen (#308 R5a): kavach's session answer gains a sealed,
// HttpOnly, /v1/t-scoped, one-hour cookie carrying it — never the database
// (forge-token-cookie.ts). The request is cloned because kavach reads it.
export const handle: Handle = async ({ event, resolve }) => {
	const sync =
		event.request.method === 'POST' && event.url.pathname === kavachConfig.routes.session
			? event.request.clone()
			: null;
	const response = (await kavach.handle({ event, resolve })) as Response;
	return sync ? captureForgeToken(sync, response, serverSecret()) : response;
};
