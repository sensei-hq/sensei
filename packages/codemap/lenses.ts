// Lens views of L0 (M17), pure: which bands a lens draws, and the Matrix.
import type { Band, Block, Coupling, Level0, ZonePoint, Zones } from './types';
import { groupOrder } from './layout';

/** The modules in each cycle, as a lookup: module → its cycle's index. */
export function cycleOf(l0: Level0): Map<string, number> {
  const out = new Map<string, number>();
  (l0.cycles as { members?: string[] }[]).forEach((c, i) => (c.members ?? []).forEach((m) => out.set(m, i)));
  return out;
}

/** Cycles lens: only the bands inside one cycle. */
export function cycleBands(l0: Level0): Band[] {
  const of = cycleOf(l0);
  return l0.bands.filter((b) => of.has(b.source) && of.get(b.source) === of.get(b.target));
}

export type MatrixIssue = 'mutual' | 'up' | 'skip';

export interface Matrix {
  ids: string[];
  /** weight[i][j]: row i depends on column j. */
  weight: number[][];
  upward: boolean[][];
  /** What is wrong with a cell: the pair depends both ways, it points up a layer,
   *  or it skips one; null for a healthy (or empty) cell. */
  issue: (MatrixIssue | null)[][];
  /** Mutual pairs, upward cells and skip cells — the legend's counts. */
  issues: Record<MatrixIssue, number>;
  max: number;
}

/** The module × module matrix, ordered by group (top layer first) then size,
 *  so an upward dependency sits below the diagonal. */
export function matrix(l0: Level0): Matrix {
  const order = groupOrder(l0.blocks, l0.groupBy);
  const ids = [...l0.blocks]
    .filter((b) => !b.hub)
    .sort((a, b) => order.indexOf(a.group) - order.indexOf(b.group) || b.symbols - a.symbols)
    .map((b) => b.id);
  const index = new Map(ids.map((id, i) => [id, i]));
  const weight = ids.map(() => ids.map(() => 0));
  const upward = ids.map(() => ids.map(() => false));
  const skip = ids.map(() => ids.map(() => false));
  let max = 0;
  for (const b of l0.bands) {
    const [i, j] = [index.get(b.source), index.get(b.target)];
    if (i === undefined || j === undefined) continue;
    weight[i][j] += b.weight;
    upward[i][j] ||= b.direction === 'up';
    skip[i][j] ||= b.direction === 'skip';
    max = Math.max(max, weight[i][j]);
  }
  const issues: Record<MatrixIssue, number> = { mutual: 0, up: 0, skip: 0 };
  const issue = ids.map((_, i) =>
    ids.map((_, j): MatrixIssue | null => {
      if (!weight[i][j] || i === j) return null;
      if (weight[j][i]) {
        if (i < j) issues.mutual += 1;
        return 'mutual';
      }
      if (upward[i][j]) return (issues.up += 1), 'up';
      if (skip[i][j]) return (issues.skip += 1), 'skip';
      return null;
    }),
  );
  return { ids, weight, upward, issue, issues, max };
}

/** What the Zones filter explores: pain and uselessness together (the default),
 *  one zone, or every placed component. */
export type ZoneFilter = 'issues' | 'all' | NonNullable<ZonePoint['zone']>;

/** Whether a component in `zone` is in the filter's focus. */
export function inZoneFilter(zone: ZonePoint['zone'], filter: ZoneFilter): boolean {
  if (zone === null) return false;
  if (filter === 'all') return true;
  if (filter === 'issues') return zone === 'pain' || zone === 'useless';
  return zone === filter;
}

export interface ZonesView {
  placed: ZonePoint[];
  unplaceable: number;
  /** The components furthest from the main sequence — what to look at. */
  furthest: ZonePoint[];
  byZone: Record<'main' | 'pain' | 'useless' | 'risk', number>;
}

/** Furthest from the main sequence first; a point that cannot be placed has no distance and sorts last. */
export const byDistance = (a: ZonePoint, b: ZonePoint): number => (b.distance ?? -1) - (a.distance ?? -1);

/** A point's distance as the lenses print it; a dash when it cannot be placed. */
export const distanceText = (p: ZonePoint): string => (p.distance === null ? '—' : p.distance.toFixed(2));

/** The Zones lens (M17): what can be placed, how many cannot, the furthest. */
export function zonesView(z: Zones, top = 10): ZonesView {
  const placed = z.points.filter((p) => p.zone !== null);
  const byZone = { main: 0, pain: 0, useless: 0, risk: 0 };
  for (const p of placed) byZone[p.zone!] += 1;
  const furthest = [...placed].sort((a, b) => byDistance(a, b) || a.component.localeCompare(b.component)).slice(0, top);
  return { placed, unplaceable: z.points.length - placed.length, furthest, byZone };
}

/** The Hidden coupling lens (M17): each coupling as a band between the blocks
 *  that hold its two modules (a folded module is drawn as its "+N more");
 *  couplings inside one block, or naming a module the map lacks, are not drawn. */
export function couplingBands(l0: Level0, couplings: Coupling[]): Band[] {
  const drawnAs = new Map<string, string>();
  for (const b of l0.blocks) {
    drawnAs.set(b.id, b.id);
    for (const m of b.folded) drawnAs.set(m, b.id);
  }
  const bands = new Map<string, Band>();
  for (const c of couplings) {
    const [s, t] = [drawnAs.get(c.a), drawnAs.get(c.b)];
    if (!s || !t || s === t) continue;
    const key = s < t ? `${s}|${t}` : `${t}|${s}`;
    const band = bands.get(key) ?? { source: s, target: t, weight: 0, kinds: {}, direction: null };
    band.weight += c.together;
    band.kinds.cochange = band.weight;
    bands.set(key, band);
  }
  return [...bands.values()].sort((a, b) => b.weight - a.weight);
}

/** The Complexity lens (M17): a block's share of functions over cyclomatic 10;
 *  null when it was not measured — never 0, which would read as simple. */
export function complexityShare(b: Block): number | null {
  if (!b.complexity) return null;
  return b.complexity.functions ? b.complexity.complex / b.complexity.functions : 0;
}
