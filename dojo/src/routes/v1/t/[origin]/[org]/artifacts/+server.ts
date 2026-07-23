// POST/GET /v1/t/{origin}/{org}/artifacts — the tenant's artifact federation:
// the contribute → triage → approve → distribute loop (Plane A). The in-Worker
// port of dojo-mind's `publish_artifact` / `pull_artifacts` (api.rs), reusing the
// device-token auth plane (`resolveApiKeyAccess`): the daemon's `DojoClient`
// points its `{registry_url}/v1/t/{tenant_key}/artifacts` here and presents its
// Keychain device token as `Authorization: Bearer …`. The wire shape matches
// `dojo_protocol` (PublishArtifactResponse / ArtifactPullResponse) so senseid's
// dojo client (`crates/senseid/src/dojo/client.rs`) retargets with only a
// URL/auth swap. See docs/plan/2026-07-23-retire-dojo-mind.md (D1 option a).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import {
	parsePublishedArtifact,
	publishArtifact,
	pullArtifactsSince,
	promoteCluster,
	ArtifactError
} from '$lib/server/artifacts-data';

// Daemon → publish (contribute) an artifact under this tenant, then run the
// inline collective-promote step (cluster + k-anonymity gate + on-threshold
// promotion). Contributor+ (mirrors dojo-mind's `DojoAccess::Contributor`
// floor). Attribution (`contributed_by`) comes from the authenticated caller,
// never the body — a caller can't spoof who contributed.
export const POST: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const artifact = parsePublishedArtifact(body);
		// Rejects a missing/blank required field OR a payload whose `kind` tag
		// disagrees with the envelope `kind` (dojo-mind's `kind_matches_payload`).
		if (!artifact) return apiError(400, 'artifact kind does not match payload or a required field is missing');

		const db = dojoDb();
		const resp = await publishArtifact(db, caller.tenantId, artifact, caller.userId);

		// Close the loop inline: triage this signature-cluster (auto-publish if it
		// clears the bar, else queue for a maintainer). Promotion failure must NOT
		// fail the already-committed contribution — surface it in the log instead
		// (a later publish or a maintainer promote sweep re-runs it idempotently),
		// mirroring dojo-mind's swallowed-but-logged promotion on the publish path.
		try {
			await promoteCluster(db, caller.tenantId, artifact.signature);
		} catch (e) {
			const message = e instanceof Error ? e.message : String(e);
			console.error(`collective promotion failed after publish (${artifact.signature}): ${message}`);
		}
		return Response.json(resp);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof ArtifactError) return apiError(e.status, e.message);
		throw e;
	}
};

// Daemon → pull artifact deltas since a cursor. Member+ (any authenticated
// device token for this tenant may pull, mirroring dojo-mind's `pull_artifacts`).
// Strictly scoped to the path tenant — a tenant never sees another's artifacts.
export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.member);
		const sinceRaw = url.searchParams.get('since');
		const parsed = sinceRaw == null ? 0 : Number.parseInt(sinceRaw, 10);
		const since = Number.isFinite(parsed) ? parsed : 0;
		const page = await pullArtifactsSince(dojoDb(), caller.tenantId, since);
		return Response.json(page);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof ArtifactError) return apiError(e.status, e.message);
		throw e;
	}
};
