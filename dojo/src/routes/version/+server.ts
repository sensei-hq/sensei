// GET /version — returns the build-time stamp (version · gitSha · builtAt) so a
// live deploy is verifiable: `curl https://dojo.sensei-hq.com/version`. Reads the
// single source in src/lib/version.ts (same values shown in the console footer).
//
// This is a PUBLIC route: it must be reachable unauthenticated. `/version` is not
// covered by the `/`, `/signin`, `/v1`, `/orgs` or `/console` rules in
// kavach.config.js, and the kavach/sentry guard redirects any unmatched path to
// /signin — so an explicit `{ path: '/version', public: true }` rule was added
// there to exempt it.
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { dojoBuild } from '$lib/version';

export const GET: RequestHandler = () => json(dojoBuild);
