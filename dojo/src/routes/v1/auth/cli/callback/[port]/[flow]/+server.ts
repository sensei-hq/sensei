// GET /v1/auth/cli/callback/{port}/{flow} — a NAMED daemon sign-in comes back
// here (#294), and the browser is bounced to the daemon's loopback with the flow
// id beside the code, so a daemon with two sign-ins in flight pairs each code
// with its own verifier. The unnamed form (../+server.ts) stays for daemons that
// do not name their flows.
import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { apiError } from '$lib/server/dojo-auth';
import { daemonRedirect, isFlowId, isForwardablePort } from '$lib/server/cli-auth';

export const GET: RequestHandler = ({ params, url }) => {
	if (!isForwardablePort(params.port)) {
		return apiError(400, 'port must be an integer in 1024–65535');
	}
	if (!isFlowId(params.flow)) return apiError(400, 'flow must be 16–64 of [A-Za-z0-9_-]');
	redirect(303, daemonRedirect(Number(params.port), url.searchParams, params.flow));
};
