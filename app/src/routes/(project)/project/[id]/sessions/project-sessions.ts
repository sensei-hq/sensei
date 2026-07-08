// Project-scoped Sessions — the one piece of wiring the project screen adds
// on top of the shared digest. Everything else (charts, rows, per-day
// aggregation, quality tones, range/chart state) is reused from
// $lib/sessions-digest.* and $lib/components/sessions.
import type { SenseiApi } from '$lib/api.js';
import type { SessionsFetcher } from '$lib/sessions-digest.svelte.js';

/** Range-fetcher scoped to one project: passes the id through to
 *  `/api/sessions?project=<id>&range=<range>`. The endpoint already accepts
 *  both params additively, so no per-project aggregate endpoint is needed. */
export function projectSessionsFetcher(
  api: Pick<SenseiApi, 'getSessionsDigest'>,
  projectId: string,
): SessionsFetcher {
  return (range) => api.getSessionsDigest(range, projectId).then((d) => d.sessions);
}
