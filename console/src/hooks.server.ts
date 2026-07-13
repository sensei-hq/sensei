import { kavach } from '$kavach/auth';
import type { Handle } from '@sveltejs/kit';

// Mirror sites/demo/src/hooks.server.js: the kavach instance's `.handle`
// (built on @kavach/sentry) reads the session cookie into `event.locals.session`
// and enforces the `rules` from kavach.config.js — guarding `/console/*` (and
// `/orgs`) so an unauthenticated request is redirected to `/signin`.
export const handle: Handle = ({ event, resolve }) =>
	kavach.handle({ event, resolve }) as ReturnType<Handle>;
