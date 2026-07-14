// Observatory · Traceability — cross-project doc-drift derivations and copy.
// The daemon owns per-project drift detection; this module only fans the
// per-project rows into a rollup, groups them by project (broken-first), and
// maps the drift status discriminator to named-token chip classes + copy.
//
// The full traceability spec (docs/spec/screen/observatory-traceability.md)
// also wants a per-row confidence chip and an expected-vs-actual diff. The wire
// `DriftItem` carries neither today (`confidence` is absent and both signatures
// come back null), so those are deliberately NOT synthesized here — see the
// page component's follow-up note. Pure functions + types only; no runes.
import type { DriftItem } from '$lib/types.js';

/** Semantic tone a drift status maps to. `ink` = an unexpected status we still
 *  surface rather than drop (never a status colour). */
export type DriftTone = 'danger' | 'warning' | 'ink';

/** One cross-project drift row — a wire `DriftItem` annotated with the project
 *  it belongs to so the observatory-wide list can group + label it. */
export interface DriftRow {
  id: string;
  projectId: string;
  projectName: string;
  /** Human-readable drift note, e.g. "Mentions `foo` which is not in the code." */
  detail: string;
  /** Wire status — `broken` (target gone) or `drifted` (signature changed). */
  status: string;
}

/** Drift rows for one project, plus that project's per-status counts. */
export interface DriftGroup {
  projectId: string;
  projectName: string;
  rows: DriftRow[];
  broken: number;
  drifted: number;
}

/** Cross-project rollup shown in the header line. */
export interface DriftRollup {
  total: number;
  broken: number;
  drifted: number;
  /** Distinct projects that have at least one drift row. */
  projectsAffected: number;
}

/** Chip utility classes for a drift status — all named tokens. */
export interface ChipClasses {
  bg: string;
  text: string;
}

/** Pure: annotate a wire `DriftItem` with its owning project. Kept next to the
 *  loader so the two stay in lockstep. */
export function toDriftRow(
  item: DriftItem,
  project: { id: string; name: string },
): DriftRow {
  return {
    id: item.id,
    projectId: project.id,
    projectName: project.name,
    detail: item.detail ?? '',
    status: item.status,
  };
}

/** Pure: drift status → semantic tone. `broken` (the referenced symbol no
 *  longer exists) is the harder failure and reads danger; `drifted` (signature
 *  changed) reads warning; anything else degrades to muted ink. */
export function driftTone(status: string): DriftTone {
  if (status === 'broken') return 'danger';
  if (status === 'drifted') return 'warning';
  return 'ink';
}

/** Pure: named-token chip classes for a drift status. The lookup IS the
 *  rendering rule, so it lives with the domain rather than in the component. */
export function driftChipClasses(status: string): ChipClasses {
  switch (driftTone(status)) {
    case 'danger':
      return { bg: 'bg-danger-soft', text: 'text-danger' };
    case 'warning':
      return { bg: 'bg-warning-soft', text: 'text-warning' };
    default:
      return { bg: 'bg-paper-mute', text: 'text-ink-mute' };
  }
}

/** Pure: cross-project rollup counts over every drift row. Plain object
 *  bookkeeping (not Set/Map) keeps the module non-reactive — it runs in the
 *  `+page.ts` load path, never as component state. */
export function rollupDrift(rows: DriftRow[]): DriftRollup {
  const seen: Record<string, true> = Object.create(null);
  let broken = 0;
  let drifted = 0;
  let projectsAffected = 0;
  for (const r of rows) {
    if (!seen[r.projectId]) {
      seen[r.projectId] = true;
      projectsAffected++;
    }
    if (r.status === 'broken') broken++;
    else if (r.status === 'drifted') drifted++;
  }
  return { total: rows.length, broken, drifted, projectsAffected };
}

/** Rank a status for broken-first ordering — broken leads, drifted next, any
 *  other status trails but is never dropped. */
function statusRank(status: string): number {
  if (status === 'broken') return 0;
  if (status === 'drifted') return 1;
  return 2;
}

/** Pure: group drift rows by project. Groups are ordered broken-first (most
 *  broken refs lead, then most total, then project name); within each group
 *  broken rows lead, then drifted, ties broken by detail for determinism. */
export function groupDriftByProject(rows: DriftRow[]): DriftGroup[] {
  // Plain object + insertion-ordered array (not Map) — this module is a pure
  // load-path helper, so it stays free of Svelte's reactive collections.
  const byProject: Record<string, DriftGroup> = Object.create(null);
  const groups: DriftGroup[] = [];
  for (const r of rows) {
    let g = byProject[r.projectId];
    if (!g) {
      g = { projectId: r.projectId, projectName: r.projectName, rows: [], broken: 0, drifted: 0 };
      byProject[r.projectId] = g;
      groups.push(g);
    }
    g.rows.push(r);
    if (r.status === 'broken') g.broken++;
    else if (r.status === 'drifted') g.drifted++;
  }
  for (const g of groups) {
    g.rows.sort(
      (a, b) => statusRank(a.status) - statusRank(b.status) || a.detail.localeCompare(b.detail),
    );
  }
  groups.sort(
    (a, b) =>
      b.broken - a.broken ||
      b.rows.length - a.rows.length ||
      a.projectName.localeCompare(b.projectName),
  );
  return groups;
}

/** Pure: "N broken references" style count with singular/plural agreement. */
function plural(n: number, one: string): string {
  return `${n} ${n === 1 ? one : `${one}s`}`;
}

/** Pure: the rollup header line, e.g. "153 broken references across 4 projects."
 *  Sentence case, no exclamation — voice charter. */
export function rollupHeadline(r: DriftRollup): string {
  const parts: string[] = [];
  if (r.broken > 0) parts.push(plural(r.broken, 'broken reference'));
  if (r.drifted > 0) parts.push(plural(r.drifted, 'drifted reference'));
  const refs = parts.length > 0 ? parts.join(' and ') : plural(r.total, 'reference');
  return `${refs} across ${plural(r.projectsAffected, 'project')}.`;
}
