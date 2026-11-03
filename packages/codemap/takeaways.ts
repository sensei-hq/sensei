// What to act on, read off the map (M17), pure. Each diagram is a facet (user,
// 2026-11-02): it lists what to act on for what it draws, and nothing else —
// Structure the layering, Cycles the cycles, the Matrix the cells to clear, Zones
// pain and uselessness, Complexity the complex modules, Hidden coupling the pairs,
// the Wheel fan-in and fan-out. Each item names a module to open.
import type { Tone } from './inspector';
import type { Lens } from './state.svelte';
import { complexityTier } from './view';
import { byDistance, distanceText } from './lenses';
import type { Band, CoChange, ComplexityNode, ComplexityTree, Level0, Zones } from './types';

export interface Takeaway {
  title: string;
  /** Why it matters and what fixing it looks like. */
  detail: string;
  tone: Tone;
  items: {
    /** The item as one line — what Copy puts in the markdown. */
    text: string;
    value: string;
    /** The module to focus; null for none. */
    target: string | null;
    /** The modules it names, one by one, when it names more than one (user, 2026-11-03). */
    paths?: string[];
    /** How those modules relate: depends on, depend on each other, change together. */
    join?: '→' | '⇄' | '⇢';
  }[];
}

/** Items listed under one takeaway. */
const SHOWN = 4;
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const heaviest = (bands: Band[]) => [...bands].sort((a, b) => b.weight - a.weight).slice(0, SHOWN);
const bandItems = (bands: Band[]) =>
  heaviest(bands).map((b) => ({ text: `${b.source} → ${b.target}`, value: String(b.weight), target: b.source, paths: [b.source, b.target], join: '→' as const }));

const UPWARD_WHY = 'A lower layer calling up is a layering violation. Put an interface below and depend on it, or move the code to the layer that uses it.';

export function takeaways(
  lens: Lens,
  l0: Level0,
  cochange: CoChange | null,
  zones: Zones | null,
  tree: ComplexityTree | null = null,
): Takeaway[] {
  switch (lens) {
    case 'structure': return structure(l0);
    case 'cycles': return cycles(l0);
    case 'matrix': return matrixIssues(l0);
    case 'zones': return zoneIssues(zones);
    case 'complexity': return tree ? complexFiles(tree) : complexity(l0);
    case 'coupling': return coupling(cochange);
    case 'wheel': return wheel(l0);
  }
}

function upward(l0: Level0): Takeaway[] {
  const up = l0.bands.filter((b) => b.direction === 'up');
  return up.length
    ? [{ title: plural(up.length, 'upward dependency', 'upward dependencies'), detail: UPWARD_WHY, tone: 'danger', items: bandItems(up) }]
    : [];
}

function structure(l0: Level0): Takeaway[] {
  const skips = l0.bands.filter((b) => b.direction === 'skip');
  return [
    ...upward(l0),
    ...(skips.length
      ? [{
          title: plural(skips.length, 'layer skip', 'layer skips'),
          detail: 'Not a violation: a layer bypassed. Worth a look where it is heavy, as the skipped layer may be missing a piece.',
          tone: 'warning' as Tone,
          items: bandItems(skips),
        }]
      : []),
  ];
}

function cycles(l0: Level0): Takeaway[] {
  const all = (l0.cycles as { members?: string[] }[]).map((c) => c.members ?? []).filter((m) => m.length);
  if (!all.length) return [];
  return [{
    title: `Break ${plural(all.length, 'cycle', 'cycles')}`,
    detail: 'Modules that depend on themselves through others cannot be split, layered or released apart. Cut the lightest dependency in each: move the piece it needs down, or invert it behind an interface.',
    tone: 'warning',
    items: all.slice(0, SHOWN).map((members) => {
      const inside = new Set(members);
      const cut = l0.bands
        .filter((b) => inside.has(b.source) && inside.has(b.target))
        .sort((a, b) => a.weight - b.weight)[0];
      return { text: members.join(' ⇄ '), value: cut ? `cut ${cut.source} → ${cut.target} (${cut.weight})` : '', target: members[0], paths: members, join: '⇄' as const };
    }),
  }];
}

function matrixIssues(l0: Level0): Takeaway[] {
  const weight = new Map(l0.bands.map((b) => [`${b.source}|${b.target}`, b.weight]));
  const mutual = l0.bands
    .filter((b) => b.source < b.target && weight.has(`${b.target}|${b.source}`))
    .map((b) => ({ b, back: weight.get(`${b.target}|${b.source}`)! }))
    .sort((x, y) => y.b.weight + y.back - (x.b.weight + x.back));
  return [
    ...(mutual.length
      ? [{
          title: plural(mutual.length, 'mutual dependency', 'mutual dependencies'),
          detail: 'Two modules that depend on each other: a cell on each side of the diagonal. One of the two directions has to go.',
          tone: 'danger' as Tone,
          items: mutual.slice(0, SHOWN).map(({ b, back }) => ({ text: `${b.source} ⇄ ${b.target}`, value: `${b.weight} · ${back}`, target: b.source, paths: [b.source, b.target], join: '⇄' as const })),
        }]
      : []),
    ...upward(l0),
  ];
}

function zoneIssues(zones: Zones | null): Takeaway[] {
  const placed = zones?.points.filter((p) => p.zone !== null) ?? [];
  const pain = placed.filter((p) => p.zone === 'pain').sort((a, b) => b.ca - a.ca);
  const useless = placed.filter((p) => p.zone === 'useless').sort(byDistance);
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
          items: useless.slice(0, SHOWN).map((p) => ({ text: p.component, value: `distance ${distanceText(p)}`, target: p.component })),
        }]
      : []),
  ];
}

function complexity(l0: Level0): Takeaway[] {
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

/** The Complexity tree's files with functions over cyclomatic 10, most first,
 *  each opening its module. */
function complexFiles(tree: ComplexityTree): Takeaway[] {
  const files: { node: ComplexityNode; module: string }[] = [];
  const walk = (n: ComplexityNode, module: string) =>
    n.kind === 'file' ? n.complex > 0 && files.push({ node: n, module }) : n.children.forEach((c) => walk(c, module));
  tree.modules.forEach((m) => walk(m, m.id));
  files.sort((a, b) => b.node.complex - a.node.complex || b.node.maxCyclo - a.node.maxCyclo || a.node.id.localeCompare(b.node.id));
  return files.length
    ? [{
        title: plural(files.length, 'file with functions over cyclomatic 10', 'files with functions over cyclomatic 10'),
        detail: 'Split the long, branchy functions in these first, and test them before you do: complexity is where defects hide and changes go wrong.',
        tone: 'danger',
        items: files.slice(0, SHOWN).map(({ node, module }) => ({
          text: node.id,
          value: `${node.complex} of ${node.functions} · max ${node.maxCyclo}`,
          target: module,
        })),
      }]
    : [];
}

function coupling(cochange: CoChange | null): Takeaway[] {
  const pairs = [...(cochange?.hidden ?? [])].sort((a, b) => b.together - a.together);
  return pairs.length
    ? [{
        title: plural(pairs.length, 'hidden coupling', 'hidden couplings'),
        detail: 'Changed together with no dependency between them: a shared assumption or a copy. Make the dependency explicit, or extract what they share.',
        tone: 'warning',
        items: pairs.slice(0, SHOWN).map((c) => ({ text: `${c.a} ⇢ ${c.b}`, value: `${c.together} commits`, target: c.a, paths: [c.a, c.b], join: '⇢' as const })),
      }]
    : [];
}

function wheel(l0: Level0): Takeaway[] {
  const ranked = (key: 'weight_in' | 'weight_out') =>
    l0.blocks
      .filter((b) => !b.folded.length && b[key] > 0)
      .sort((a, b) => b[key] - a[key] || a.id.localeCompare(b.id))
      .slice(0, SHOWN)
      .map((b) => ({ text: b.id, value: String(b[key]), target: b.id }));
  const fanIn = ranked('weight_in');
  const fanOut = ranked('weight_out');
  return [
    ...(fanIn.length
      ? [{ title: 'Most depended on', detail: 'The modules the most code leans on. Keep them stable, small and well tested; a change here reaches far.', tone: 'ink' as Tone, items: fanIn }]
      : []),
    ...(fanOut.length
      ? [{ title: 'Widest reach', detail: 'The modules that depend on the most. Often an entry point; elsewhere, a module doing too much.', tone: 'ink' as Tone, items: fanOut }]
      : []),
  ];
}

/** What to act on as markdown, for the Copy action. */
export function takeawaysMarkdown(lensLabel: string, items: Takeaway[]): string {
  const lines = [`## What to act on · ${lensLabel}`, ''];
  for (const t of items) {
    lines.push(`### ${t.title}`, '', t.detail, '');
    for (const i of t.items) lines.push(`- ${i.text}${i.value ? ` — ${i.value}` : ''}`);
    lines.push('');
  }
  return lines.join('\n');
}
