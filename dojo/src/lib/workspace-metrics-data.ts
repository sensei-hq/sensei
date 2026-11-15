// One workspace's metric series, as the drill-ins read it (WP5): the wire
// behind GET /v1/t/{origin}/{org}/workspaces/{id}/metrics (WP4) plus the fetch.
// Tenant-addressed; the caller reads as themselves (a workspace viewer without
// a membership included). A failed read throws, so the screen can say it
// rather than show an empty window.
import { authHeaders, dojoApiUrl, encodeTenant, parseJson, type DojoCallOpts } from './dojo-api';

/** One series row on the wire (mirrors the server `WorkspaceMetricRow`).
 *  `scope` is `repo` (the whole-tree twin), `user` (per-person detail the
 *  caller may read), or `pooled` (the viewer aggregate, principal null). */
export interface WorkspaceMetricPoint {
	repository_id: string;
	repo_key: string;
	metric_key: string;
	scope: string;
	principal_id: string | null;
	day: string;
	value: number;
	props: unknown;
}

/** GET …/workspaces/{id}/metrics — flat rows, the per-repository split is the
 *  view's shaping (`workspace-metrics-view.ts`), not this read's. */
export interface WorkspaceMetricsWire {
	since: string;
	days: number;
	rows: WorkspaceMetricPoint[];
}

/** `GET …/workspaces/{id}/metrics[?days=]` — the workspace's series over the
 *  last `days` (the server's month when absent). Read as the caller; a failure
 *  throws, so the screen carries it as an error state, never as empty charts. */
export async function getWorkspaceMetrics(
	tenantKey: string,
	workspaceId: string,
	opts: DojoCallOpts & { days?: number } = {}
): Promise<WorkspaceMetricsWire> {
	const query = opts.days === undefined ? '' : `?days=${opts.days}`;
	const f = opts.fetch ?? fetch;
	const res = await f(
		`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/workspaces/${encodeURIComponent(workspaceId)}/metrics${query}`,
		{ headers: authHeaders(opts.accessToken) }
	);
	return parseJson<WorkspaceMetricsWire>(res);
}
