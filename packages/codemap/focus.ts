// A focus narrows every lens to the focused modules (user, 2026-11-03). The
// daemon narrows the module map itself — before it folds — and names the focus
// it applied; the lenses it does not narrow (Hidden coupling, Complexity, Zones)
// are narrowed here, from those names. Pure, so each is checked against its input.
import type { CoChange, ComplexityTree, Level0, Zones } from './types';

export interface FocusSets {
  /** The focused modules. */
  members: Set<string>;
  /** The focused modules and their direct neighbours. */
  near: Set<string>;
}

/** The focus the map was narrowed to, as sets; null when it was not. */
export function focusSets(l0: Level0 | null): FocusSets | null {
  const f = l0?.focus;
  if (!f) return null;
  const members = new Set(f.members);
  return { members, near: new Set([...f.members, ...f.neighbours]) };
}

/** Hidden coupling: the pairs a focused module is in. */
export function focusCoupling(data: CoChange | null, sets: FocusSets | null): CoChange | null {
  if (!data || !sets) return data;
  return { ...data, hidden: data.hidden.filter((c) => sets.members.has(c.a) || sets.members.has(c.b)) };
}

/** Complexity: the focused modules' branches, totalled. The maintainability
 *  reading is the workspace's, so a focus does not carry it. */
export function focusComplexity(data: ComplexityTree | null, sets: FocusSets | null): ComplexityTree | null {
  if (!data || !sets) return data;
  const modules = data.modules.filter((m) => sets.members.has(m.id));
  const total = modules.reduce(
    (t, m) => ({ functions: t.functions + m.functions, complex: t.complex + m.complex, maxCyclo: Math.max(t.maxCyclo, m.maxCyclo), files: t.files + m.files }),
    { functions: 0, complex: 0, maxCyclo: 0, files: 0 },
  );
  return { modules, total, maintainability: null };
}

/** Zones: the focused modules and their neighbours, read as the daemon reads
 *  them — the mean over the placed (none when nothing is placed: a mean over no
 *  point is no reading), `points` every point, `unplaceable` those with no distance. */
export function focusZones(data: Zones | null, sets: FocusSets | null): Zones | null {
  if (!data || !sets) return data;
  const points = data.points.filter((p) => sets.near.has(p.component));
  const distances = points.flatMap((p) => (p.distance === null ? [] : [p.distance]));
  return {
    points,
    meanDistance: distances.length ? distances.reduce((a, d) => a + d, 0) / distances.length : null,
    coverage: { points: points.length, unplaceable: points.length - distances.length },
  };
}
