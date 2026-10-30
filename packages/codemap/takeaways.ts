// What to act on, read off the map (M17), pure: at most four takeaways, in the
// order to work them — break the cycles, invert the upward dependencies, then the
// lens's own (the zone of pain, the most complex modules, the strongest hidden
// coupling, or the layer skips). Each item names a module to open.
import type { Tone } from './inspector';
import type { Lens } from './state.svelte';
import { complexityTier } from './view';
import type { Band, CoChange, Level0, Zones } from './types';

export interface Takeaway {
  title: string;
  /** Why it matters and what fixing it looks like. */
  detail: string;
  tone: Tone;
  items: { text: string; value: string; /** The module to open; null for none. */ target: string | null }[];
}

/** Items listed under one takeaway. */
const SHOWN = 4;
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const heaviest = (bands: Band[]) => [...bands].sort((a, b) => b.weight - a.weight).slice(0, SHOWN);

export function takeaways(lens: Lens, l0: Level0, cochange: CoChange | null, zones: Zones | null): Takeaway[] {
  const out: Takeaway[] = [];
  const cycles = (l0.cycles as { members?: string[] }[]).map((c) => c.members ?? []).filter((m) => m.length);
  if (cycles.length) {
    out.push({
      title: `Break ${plural(cycles.length, 'cycle', 'cycles')}`,
      detail: 'Modules that depend on themselves through others cannot be split, layered or released apart. Break these first: move the shared piece down, or invert one dependency.',
      tone: 'warning',
      items: cycles.slice(0, SHOWN).map((m) => ({ text: m.join(' ⇄ '), value: '', target: m[0] })),
    });
  }
  const upward = l0.bands.filter((b) => b.direction === 'up');
  if (upward.length) {
    out.push({
      title: plural(upward.length, 'upward dependency', 'upward dependencies'),
      detail: 'A lower layer calling up is a layering violation. Put an interface below and depend on it, or move the code to the layer that uses it.',
      tone: 'danger',
      items: heaviest(upward).map((b) => ({ text: `${b.source} → ${b.target}`, value: String(b.weight), target: b.source })),
    });
  }
  out.push(...lensTakeaways(lens, l0, cochange, zones));
  return out.slice(0, 4);
}

function lensTakeaways(lens: Lens, l0: Level0, cochange: CoChange | null, zones: Zones | null): Takeaway[] {
  if (lens === 'zones') {
    const placed = zones?.points.filter((p) => p.zone !== null) ?? [];
    const pain = placed.filter((p) => p.zone === 'pain').sort((a, b) => b.ca - a.ca);
    const useless = placed.filter((p) => p.zone === 'useless').sort((a, b) => b.distance - a.distance);
    return [
      ...(pain.length
        ? [{
            title: `${plural(pain.length, 'module', 'modules')} in the zone of pain`,
            detail: 'Concrete and depended on, so every change ripples. Where one holds logic, put an interface in front of it; a module of plain data or types is expected here.',
            tone: 'danger' as Tone,
            items: pain.slice(0, SHOWN).map((p) => ({ text: p.component, value: plural(p.ca, 'dependent', 'dependents'), target: p.component })),
          }]
        : []),
      ...(useless.length
        ? [{
            title: plural(useless.length, 'abstraction nothing uses', 'abstractions nothing uses'),
            detail: 'Interfaces or traits with little depending on them. Remove them, or use them where the concrete type is used now.',
            tone: 'warning' as Tone,
            items: useless.slice(0, SHOWN).map((p) => ({ text: p.component, value: `distance ${p.distance.toFixed(2)}`, target: p.component })),
          }]
        : []),
    ];
  }
  if (lens === 'complexity') {
    const hot = l0.blocks
      .filter((b) => !b.folded.length && complexityTier(b).tone === 'danger')
      .sort((a, b) => (complexityTier(b).share ?? 0) - (complexityTier(a).share ?? 0));
    return hot.length
      ? [{
          title: `${plural(hot.length, 'module', 'modules')} over 20% complex`,
          detail: 'More than one function in five is over cyclomatic complexity 10. Split those functions, and test them first.',
          tone: 'danger',
          items: hot.slice(0, SHOWN).map((b) => ({ text: b.id, value: `${Math.round((complexityTier(b).share ?? 0) * 100)}%`, target: b.id })),
        }]
      : [];
  }
  if (lens === 'coupling') {
    const pairs = [...(cochange?.hidden ?? [])].sort((a, b) => b.together - a.together);
    return pairs.length
      ? [{
          title: plural(pairs.length, 'hidden coupling', 'hidden couplings'),
          detail: 'Changed together with no dependency between them: a shared assumption or a copy. Make the dependency explicit, or extract what they share.',
          tone: 'warning',
          items: pairs.slice(0, SHOWN).map((c) => ({ text: `${c.a} ⇢ ${c.b}`, value: `${c.together} commits`, target: c.a })),
        }]
      : [];
  }
  const skips = l0.bands.filter((b) => b.direction === 'skip');
  return skips.length
    ? [{
        title: plural(skips.length, 'layer skip', 'layer skips'),
        detail: 'Not a violation: a layer bypassed. Worth a look where it is heavy, as the skipped layer may be missing a piece.',
        tone: 'warning',
        items: heaviest(skips).map((b) => ({ text: `${b.source} → ${b.target}`, value: String(b.weight), target: b.source })),
      }]
    : [];
}
