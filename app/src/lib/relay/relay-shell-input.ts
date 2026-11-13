// From what the screen holds — the read, its state, the person's view — to the
// shell's input (`toRelayShell`). Pure. The four stats are the relay selectors'
// (`toStats`, @sensei/state), counted from the read the daemon gave; a number it did
// not give passes as null and reads "not reported" downstream, never 0.

import { firstPassLine, minutesSince, toStats } from '@sensei/state';
import type { RelayNow } from '$lib/relay-api.js';
import { crewTail } from './relay-format.js';
import { WAIT_THRESHOLD_MIN, type RelayShellInput, type RelayStatFilter, type RelaySurface } from './relay-shell.js';

export interface ShellSource {
  surface: RelaySurface;
  /** The place the context names: the workspace, or "all workspaces". */
  where: string;
  now: RelayNow | null;
  /** The last read failed (the body carries the Notice). */
  failed: boolean;
  reading: boolean;
  view: 'now' | 'queue' | 'crew';
  composing: boolean;
  statFilter: RelayStatFilter | null;
  queueCount: number | null;
  /** The main window's URL narrowing ('' = all); absent in the workspace window. */
  narrow?: { repo: string; workspace: string } | null;
  /** The clock, for ages; defaults to now. */
  at?: number;
}

export function toShellInput(src: ShellSource): RelayShellInput {
  const at = src.at ?? Date.now();
  const now = src.now;
  const load: RelayShellInput['load'] = src.reading ? 'loading' : src.failed || !now ? 'error' : now.crews.length === 0 ? 'empty' : 'loaded';
  const run = now?.runs.find((r) => r.state === 'planning' || r.state === 'active') ?? null;

  const r = now?.resources;
  return {
    surface: src.surface,
    load,
    view: src.view,
    from: null,
    workspace: src.where,
    run: run ? { id: run.run_id.slice(-4) } : null,
    crew: now?.crews[0] ? { id: crewTail(now.crews[0].id) } : null,
    crews: now?.crews.length ?? 0,
    counts: { queue: src.queueCount ?? 0, crew: now?.lanes.length ?? 0 },
    today: now ? { done: now.today.completed, firstPass: firstPassLine(now, 'passed review first time.') } : null,
    stats: toStats(now, at),
    filter: src.statFilter,
    compose: src.composing,
    info: true,
    resources: r
      ? {
          build: r.build_lease
            ? { resource: r.build_lease.resource, lane: r.build_lease.lane_code, minutes: minutesSince(r.build_lease.granted_at, at) }
            : null,
          disk_free_gb: r.disk_free_gb,
          swap_free_gb: r.swap_free_gb,
          swap_floor_gb: r.swap_floor_gb,
        }
      : null,
    freshness: null,
    threshold: WAIT_THRESHOLD_MIN,
    narrow: src.narrow
      ? {
          repos: now?.facets.repos ?? [],
          workspaces: now?.facets.workspaces ?? [],
          repo: src.narrow.repo,
          workspace: src.narrow.workspace,
        }
      : null,
  };
}
