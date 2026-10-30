// What the map shows of L0 under the reader's choices (M17), pure: which bands
// survive the filters, how a band and a block are toned, the legend, the file
// count under the roles shown, and search.
import { complexityShare, couplingBands, cycleBands, cycleOf } from './lenses';
import { groupOrder } from './layout';
import type { Lens } from './state.svelte';
import type { Band, Block, CoChange, Direction, Level0, Role } from './types';

export type EdgeMode = 'all' | 'violations';
export interface BandFilter {
  edgeMode: EdgeMode;
  /** Bands lighter than this are not drawn. */
  minWeight: number;
}
export const MIN_WEIGHTS = [1, 10, 25] as const;

/** The bands the filters keep. "Violations & cycles" is upward, a layer skip, or a
 *  band between two members of one cycle — a same-layer band is never a violation. */
export function visibleBands(bands: Band[], filter: BandFilter, cycleOf: Map<string, number> | null): Band[] {
  const inCycle = (b: Band) =>
    !!cycleOf && cycleOf.has(b.source) && cycleOf.get(b.source) === cycleOf.get(b.target);
  return bands.filter(
    (b) =>
      b.weight >= filter.minWeight &&
      (filter.edgeMode === 'all' || b.direction === 'up' || b.direction === 'skip' || inCycle(b)),
  );
}

/** A band's colour. Upward is always red; with colour by direction on, a skip is
 *  amber and same-layer and down are told apart. Never by position. */
export function bandTone(direction: Direction | null, byDirection: boolean): string {
  if (direction === 'up') return 'var(--danger)';
  if (!byDirection) return 'var(--ink-faint)';
  return direction === 'skip' ? 'var(--warning)' : direction === 'level' ? 'var(--ink-mute)' : 'var(--ink-faint)';
}

export interface LegendEntry {
  label: string;
  /** A line (`stroke`) or a swatch (`fill`). */
  stroke?: string;
  fill?: string;
  dashed?: boolean;
}

/** What each lens's marks mean, for the strip under the canvas. */
export function legendFor(lens: Lens, byDirection: boolean): LegendEntry[] {
  switch (lens) {
    case 'cycles':
      return [{ label: 'in a cycle', stroke: 'var(--warning)' }];
    case 'coupling':
      return [{ label: 'changes together', stroke: 'var(--ink-faint)', dashed: true }];
    case 'complexity':
      return [
        { label: 'over 10%', stroke: 'var(--warning)' },
        { label: 'over 20%', stroke: 'var(--danger)' },
        { label: 'not measured', fill: 'var(--paper-mute)' },
      ];
    case 'zones':
      return [];
    default:
      return byDirection
        ? [
            { label: 'down', stroke: bandTone('down', true) },
            { label: 'same layer', stroke: bandTone('level', true) },
            { label: 'upward', stroke: bandTone('up', true) },
            { label: 'skips a layer', stroke: bandTone('skip', true) },
          ]
        : [
            { label: 'dependency', stroke: bandTone('down', false) },
            { label: 'upward', stroke: bandTone('up', false) },
          ];
  }
}

export interface ComplexityTier {
  tone: 'danger' | 'warning' | 'neutral' | 'unmeasured';
  label: string;
  /** The share over cyclomatic 10; null when not measured. */
  share: number | null;
}

/** Complexity lens: amber over 10% of functions over cyclomatic 10, red over 20%;
 *  a block never measured says so rather than reading as 0%. */
export function complexityTier(b: Block): ComplexityTier {
  const share = complexityShare(b);
  if (share === null) return { tone: 'unmeasured', label: 'not measured', share: null };
  const label = `${Math.round(share * 100)}% over cx 10`;
  return { tone: share > 0.2 ? 'danger' : share > 0.1 ? 'warning' : 'neutral', label, share };
}

/** A block's file count under the roles shown. */
export function shownFiles(b: Block, shown: Role[]): number {
  return shown.reduce((n, r) => n + (b.roles?.[r] ?? 0), 0);
}

/** The bands a lens draws under the filters: a cycle's own for Cycles, co-change
 *  for Hidden coupling (never filtered — they are not dependencies), and every
 *  band for the rest. */
export function drawnBands(lens: Lens, l0: Level0, cochange: CoChange | null, filter: BandFilter): Band[] {
  if (lens === 'coupling') return couplingBands(l0, cochange?.hidden ?? []);
  return visibleBands(lens === 'cycles' ? cycleBands(l0) : l0.bands, filter, cycleOf(l0));
}

/** What L0 draws of what there is, for the stats under the header. */
export function budgetLine(l0: Level0, bands: number): string {
  const n = (v: number) => v.toLocaleString('en-US');
  const symbols = l0.blocks.reduce((a, b) => a + b.symbols, 0);
  const unplaced = l0.coverage.unplaced ? ` · ${n(l0.coverage.unplaced)} calls unplaced` : '';
  return `drawing ${n(l0.budget.drawn)} of ${n(l0.budget.modules)} modules · ${n(bands)} bands for ${n(l0.budget.edges)} edges · ${n(symbols)} symbols${unplaced}`;
}

/** Each role's colour: the Show chips' swatches and the list rows' role bars. */
export const ROLE_BG: Record<Role, string> = {
  code: 'bg-ink-soft',
  test: 'bg-success',
  docs: 'bg-info',
  config: 'bg-warning',
  generated: 'bg-ink-faint',
};

/** Every module to browse, grouped as the map groups them: a drawn block, the
 *  modules folded into a "+N more", and hubs — so a module can be found without
 *  knowing its name. Items for a rokkit Select. */
export function moduleChoices(l0: Level0): { label: string; value: string; children: { label: string; value: string }[] }[] {
  return groupOrder(l0.blocks, l0.groupBy).map((g) => {
    const ids = l0.blocks.filter((b) => b.group === g).flatMap((b) => (b.folded.length ? b.folded : [b.id]));
    return { label: g, value: `group:${g}`, children: [...ids].sort().map((id) => ({ label: id, value: id })) };
  });
}
