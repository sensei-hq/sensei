// Lens views of L0 (M17), pure: which bands a lens draws, and the Matrix.
import type { Band, Level0 } from './types';
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

export interface Matrix {
  ids: string[];
  /** weight[i][j]: row i depends on column j. */
  weight: number[][];
  upward: boolean[][];
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
  let max = 0;
  for (const b of l0.bands) {
    const [i, j] = [index.get(b.source), index.get(b.target)];
    if (i === undefined || j === undefined) continue;
    weight[i][j] += b.weight;
    upward[i][j] ||= b.direction === 'up';
    max = Math.max(max, weight[i][j]);
  }
  return { ids, weight, upward, max };
}
