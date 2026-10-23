// POST /v1/internal/azure/member-sync — the scheduled Azure DevOps member read
// (#308 R5b, D44): reads up to a batch of connected organizations' members and
// applies #309's rule (azure-member-sync.ts). The Worker's cron trigger calls it
// in-process (worker-entry.js); it is not public: only a request bearing the
// server secret DOJO_CRON_SECRET (at least 32 characters; compared in constant
// time) runs it — 401 otherwise, 503 when the secret is missing or short, or the
// Azure app registration is not configured. Answers what the run came to; a run that cannot read the
// connections at all is an error answer.
import type { RequestHandler } from './$types';
import { env as priv } from '$env/dynamic/private';
import { dojoDb } from '$lib/server/dojo-supabase';
import { azureEntraConfig } from '$lib/server/azure-entra';
import { azureAdapter } from '$lib/server/forge-azure';
import { runAzureMemberSync } from '$lib/server/azure-member-sync';

const json = (status: number, error: string) => Response.json({ error }, { status });

/** DOJO_CRON_SECRET must be at least this long (#308 R5b review 2). */
const MIN_SECRET_LENGTH = 32;

/** Constant-time string equality (no early exit on the first difference). */
function sameSecret(a: string, b: string): boolean {
	const x = new TextEncoder().encode(a);
	const y = new TextEncoder().encode(b);
	let diff = x.length ^ y.length;
	for (let i = 0; i < Math.max(x.length, y.length); i++) diff |= (x[i] ?? 0) ^ (y[i] ?? 0);
	return diff === 0;
}

export const POST: RequestHandler = async ({ request }) => {
	const secret = (priv.DOJO_CRON_SECRET ?? '').trim();
	// A short secret is a misconfiguration, not a weaker lock: refuse to run on it.
	if (secret.length < MIN_SECRET_LENGTH) return json(503, 'the scheduled member read is not configured on this dōjō');
	const auth = request.headers.get('authorization') ?? '';
	const presented = auth.startsWith('Bearer ') ? auth.slice('Bearer '.length) : '';
	if (!presented || !sameSecret(presented, secret)) return json(401, 'unauthenticated');

	let cfg;
	try {
		cfg = azureEntraConfig(priv);
	} catch (e) {
		return json(503, e instanceof Error ? e.message : 'Azure DevOps is not configured on this dōjō');
	}
	try {
		return Response.json(await runAzureMemberSync(dojoDb(), { cfg, adapterFor: (org) => azureAdapter(org) }));
	} catch (e) {
		const status = typeof (e as { status?: unknown }).status === 'number' ? (e as { status: number }).status : 500;
		return json(status, e instanceof Error ? e.message : 'the scheduled member read failed');
	}
};
