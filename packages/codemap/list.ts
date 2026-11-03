// Every lens as a list, for a narrow screen (M17, under 640px), pure. Modules
// grouped as the canvas groups them, with a role bar and badges; Zones ranked by
// distance from the main sequence; the Matrix as group → group dependencies.
import { groupOrder } from './layout';
import { cycleOf } from './lenses';
import type { Lens } from './state.svelte';
import { complexityTier } from './view';
import type { Block, CoChange, Level0, RoleCounts, Zones } from './types';

export interface Badge {
  text: string;
  tone: 'danger' | 'warning' | 'faint';
}
export interface ListRow {
  id: string;
  label: string;
  sub: string;
  roles: RoleCounts | null;
  badges: Badge[];
  /** Whether a tap opens it (a module) — a fold or a group pair does not. */
  open: boolean;
}
export interface ListView {
  groups: { label: string; rows: ListRow[] }[];
}

export function listView(lens: Lens, l0: Level0, cochange: CoChange | null, zones: Zones | null): ListView {
  if (lens === 'zones') return zonesList(zones);
  if (lens === 'matrix') return matrixList(l0);
  const cycles = cycleOf(l0);
  const upward = new Set(l0.bands.filter((b) => b.direction === 'up').map((b) => b.source));
  const partners = new Map<string, string[]>();
  for (const c of cochange?.hidden ?? []) {
    partners.set(c.a, [...(partners.get(c.a) ?? []), c.b]);
    partners.set(c.b, [...(partners.get(c.b) ?? []), c.a]);
  }
  const keep = (b: Block) =>
    !b.hub && (lens !== 'cycles' || cycles.has(b.id)) && (lens !== 'coupling' || partners.has(b.id));
  const rank = (a: Block, b: Block) =>
    lens === 'complexity'
      ? (complexityTier(b).share ?? -1) - (complexityTier(a).share ?? -1) || b.symbols - a.symbols
      : b.symbols - a.symbols;
  const row = (b: Block): ListRow => {
    const folded = b.folded.length > 0;
    const badges: Badge[] = [];
    if (upward.has(b.id)) badges.push({ text: 'upward', tone: 'danger' });
    if (cycles.has(b.id)) badges.push({ text: 'cycle', tone: 'warning' });
    const cx = complexityTier(b);
    if (lens === 'complexity' && cx.share !== null) badges.push({ text: cx.label, tone: cx.tone === 'danger' ? 'danger' : cx.tone === 'warning' ? 'warning' : 'faint' });
    for (const p of partners.get(b.id) ?? []) badges.push({ text: `with ${p}`, tone: 'faint' });
    return {
      id: b.id,
      label: b.label,
      sub: folded ? `${b.folded.length} smaller modules` : `${b.symbols} symbols`,
      roles: folded ? null : b.roles,
      badges: lens === 'coupling' ? badges.filter((x) => x.text.startsWith('with ')) : badges,
      open: !folded,
    };
  };
  const blocks = l0.blocks.filter(keep);
  return {
    groups: groupOrder(blocks, l0.groupBy)
      .map((g) => {
        const rows = blocks.filter((b) => b.group === g).sort(rank).map(row);
        return { label: `${g} · ${rows.length}`, rows };
      })
      .filter((g) => g.rows.length),
  };
}

function zonesList(zones: Zones | null): ListView {
  const placed = (zones?.points ?? []).filter((p) => p.zone !== null).sort((a, b) => b.distance - a.distance);
  const ZONE: Record<string, Badge | undefined> = {
    pain: { text: 'zone of pain', tone: 'danger' },
    useless: { text: 'zone of uselessness', tone: 'warning' },
  };
  return {
    groups: [{
      label: 'Furthest from the main sequence',
      rows: placed.map((p) => ({
        id: p.component,
        label: p.component,
        sub: `I ${p.instability.toFixed(2)} · A ${p.abstractness.toFixed(2)} · distance ${p.distance.toFixed(2)}`,
        roles: null,
        badges: ZONE[p.zone!] ? [ZONE[p.zone!]!] : [],
        open: true,
      })),
    }],
  };
}

function matrixList(l0: Level0): ListView {
  const groupOf = new Map(l0.blocks.map((b) => [b.id, b.group]));
  const pairs = new Map<string, { w: number; up: number }>();
  for (const b of l0.bands) {
    const key = `${groupOf.get(b.source)} → ${groupOf.get(b.target)}`;
    const p = pairs.get(key) ?? { w: 0, up: 0 };
    p.w += b.weight;
    if (b.direction === 'up') p.up += 1;
    pairs.set(key, p);
  }
  return {
    groups: [{
      label: 'Between groups · heaviest first',
      rows: [...pairs.entries()]
        .sort((a, b) => b[1].w - a[1].w)
        .map(([key, p]) => ({
          id: key,
          label: key,
          sub: `${p.w} calls + imports`,
          roles: null,
          badges: p.up ? [{ text: `${p.up} upward`, tone: 'danger' as const }] : [],
          open: false,
        })),
    }],
  };
}
