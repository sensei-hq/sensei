import { kavach } from '$kavach/auth';
import type { Handle } from '@sveltejs/kit';
import { provisionOnSignIn } from '$lib/server/provision-on-signin';

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

export const handle: Handle = ({ event, resolve }) =>
	kavach.handle({ event, resolve }) as ReturnType<Handle>;
