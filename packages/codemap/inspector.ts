// The Code map's inspector (M17), pure: what the side panel says about the
// selection at each level — a module, a hub, a fold, the referenced libraries,
// or with nothing selected the workspace by direction. L1 describes the open
// module, L2 the open file. Copy follows the mockup.
import type { Level } from './state.svelte';
import { complexityTier } from './view';
import type { Block, Direction, Level0, Level1, Level2 } from './types';

export type Tone = 'ink' | 'danger' | 'warning' | 'success' | 'faint';

export interface Inspection {
  eyebrow: string;
  title: string;
  sub: string;
  stats: { label: string; value: string; tone: Tone }[];
  listLabel: string;
  list: { text: string; value: string; tone: Tone }[];
  /** The module the panel can open into L1; null when there is nothing to open. */
  open: string | null;
}

export interface InspectInput {
  level: Level;
  l0: Level0 | null;
  l1: Level1 | null;
  l2: Level2 | null;
  /** A block id, or `libraries`. */
  selected: string | null;
}

/** The selection id of the referenced-libraries block. */
export const LIBRARIES = 'libraries';

const n = (v: number) => v.toLocaleString('en-US');
const cycleMembers = (l0: Level0) => (l0.cycles as { members?: string[] }[]).map((c) => c.members ?? []);

export function inspect({ level, l0, l1, l2, selected }: InspectInput): Inspection {
  if (level === 'symbols' && l2) return file(l2);
  if (level === 'files' && l1) return openModule(l1, l0);
  if (!l0) return empty();
  if (selected === LIBRARIES) return libraries(l0);
  const b = selected ? l0.blocks.find((x) => x.id === selected) : undefined;
  if (!b) return workspace(l0);
  if (b.folded.length) return fold(b);
  if (b.hub) return hub(b);
  return module(b, l0);
}

function module(b: Block, l0: Level0): Inspection {
  const out = l0.bands.filter((x) => x.source === b.id);
  const viol = out.filter((x) => x.direction === 'up' || x.direction === 'skip');
  const partners = cycleMembers(l0)
    .filter((m) => m.includes(b.id))
    .flatMap((m) => m.filter((id) => id !== b.id));
  const files = Object.values(b.roles).reduce((a, v) => a + v, 0);
  const cx = complexityTier(b);
  const list = [
    ...viol.map((x) => violation(x.direction, x.target, x.weight)),
    ...partners.map((id) => ({ text: `Cycle with ${id}`, value: '', tone: 'warning' as Tone })),
    ...(b.roles.docs ? [] : [{ text: 'No docs in this module', value: '', tone: 'faint' as Tone }]),
  ];
  return {
    eyebrow: ['Module', b.package, b.group].filter(Boolean).join(' · '),
    title: b.id,
    sub: "Its files split into code, tests, docs and config, so a test-heavy module doesn't read as a big one.",
    stats: [
      { label: 'Symbols', value: n(b.symbols), tone: 'ink' },
      { label: 'Files', value: n(files), tone: 'ink' },
      { label: 'Tests / code', value: (b.roles.test / Math.max(1, b.roles.code)).toFixed(2), tone: 'success' },
      { label: 'Over cx 10', value: cx.share === null ? '—' : `${Math.round(cx.share * 100)}%`, tone: cx.tone === 'danger' ? 'danger' : 'ink' },
      { label: 'Out · in', value: `${n(b.weight_out)} · ${n(b.weight_in)}`, tone: 'ink' },
      { label: 'Violations', value: String(viol.length + partners.length), tone: viol.length + partners.length ? 'danger' : 'ink' },
    ],
    listLabel: 'What to look at',
    list: viol.length + partners.length || !b.roles.docs ? list : [{ text: 'No violations or cycles', value: '', tone: 'success' }],
    open: b.id,
  };
}

function violation(direction: Direction | null, target: string, weight: number) {
  const up = direction === 'up';
  return { text: `${up ? 'Upward call to' : 'Skips a layer to'} ${target}`, value: n(weight), tone: (up ? 'danger' : 'warning') as Tone };
}

function openModule(l1: Level1, l0: Level0 | null): Inspection {
  const b = l0?.blocks.find((x) => x.id === l1.module && !x.folded.length);
  const base = b && l0 ? module(b, l0) : null;
  const viol = l1.dependsOn.filter((d) => d.direction === 'up' || d.direction === 'skip');
  const files = l1.columns.reduce((a, c) => a + c.files.length + c.more, 0);
  return {
    eyebrow: base?.eyebrow ?? 'Module',
    title: l1.module,
    sub: 'Its files by role. Open a file to see its symbols, their callers and callees.',
    stats: base?.stats ?? [
      { label: 'Files', value: n(files), tone: 'ink' },
      { label: 'Depends on', value: n(l1.dependsOn.length), tone: 'ink' },
      { label: 'Used by', value: n(l1.usedBy.length), tone: 'ink' },
      { label: 'Violations', value: String(viol.length), tone: viol.length ? 'danger' : 'ink' },
    ],
    listLabel: 'What to look at',
    list: viol.length
      ? viol.map((d) => violation(d.direction, d.module, d.weight))
      : [{ text: 'No upward or skipping dependencies', value: '', tone: 'success' }],
    open: null,
  };
}

function file(l2: Level2): Inspection {
  return {
    eyebrow: `File · ${l2.role}`,
    title: l2.path,
    sub: 'Its symbols, with what calls into it and what it calls, one hop out.',
    stats: [
      { label: 'Symbols', value: n(l2.symbols.length), tone: 'ink' },
      { label: 'Callers', value: n(l2.callersTotal), tone: 'ink' },
      { label: 'Callees', value: n(l2.calleesTotal), tone: 'ink' },
    ],
    listLabel: 'Level',
    list: [{ text: 'L2 draws one file’s symbols; the breadcrumb climbs back', value: '', tone: 'faint' }],
    open: null,
  };
}

function hub(b: Block): Inspection {
  return {
    eyebrow: 'Shared hub',
    title: b.id,
    sub: 'Called from almost everywhere. Drawing its edges would bury the map, so they are counted here and left off the canvas.',
    stats: [
      { label: 'Callers', value: n(b.weight_in), tone: 'ink' },
      { label: 'Symbols', value: n(b.symbols), tone: 'ink' },
    ],
    listLabel: 'Rule',
    list: [{ text: 'A module is a hub when its inbound edges exceed 5% of all edges', value: '', tone: 'faint' }],
    open: null,
  };
}

function fold(b: Block): Inspection {
  return {
    eyebrow: `Folded · ${b.group}`,
    title: `${b.folded.length} smaller modules`,
    sub: 'Over the block budget, the smallest modules in a band fold into one block. Search or open one by name.',
    stats: [
      { label: 'Modules', value: n(b.folded.length), tone: 'ink' },
      { label: 'Symbols', value: n(b.symbols), tone: 'ink' },
    ],
    listLabel: 'Inside',
    list: b.folded.map((id) => ({ text: id, value: '', tone: 'faint' })),
    open: null,
  };
}

function libraries(l0: Level0): Inspection {
  return {
    eyebrow: 'Referenced libraries',
    title: `${n(l0.libraries.count)} packages`,
    sub: 'Dependencies the workspace uses but does not own. Listed, never drawn as members, never gated.',
    stats: [
      { label: 'Packages', value: n(l0.libraries.count), tone: 'ink' },
      { label: 'Most used', value: l0.libraries.top[0] ?? '—', tone: 'ink' },
    ],
    listLabel: 'Most referenced',
    list: l0.libraries.top.map((t) => ({ text: t, value: '', tone: 'faint' })),
    open: null,
  };
}

export interface Stat {
  label: string;
  value: string;
  tone: Tone;
  /** The diagram that shows it, when one does. */
  lens?: 'structure' | 'cycles';
}

/** The numbers every diagram of a workspace shares — its modules and how their
 *  dependencies run — for the header's stats band. */
export function workspaceStats(l0: Level0): Stat[] {
  const count = (d: Direction) => l0.bands.filter((b) => b.direction === d).length;
  return [
    { label: 'Modules', value: n(l0.budget.modules), tone: 'ink' },
    { label: 'Symbols', value: n(l0.blocks.reduce((a, b) => a + b.symbols, 0)), tone: 'ink' },
    { label: 'Edges', value: n(l0.budget.edges), tone: 'ink' },
    { label: 'Upward', value: String(count('up')), tone: 'danger', lens: 'structure' },
    { label: 'Skip a layer', value: String(count('skip')), tone: 'warning', lens: 'structure' },
    { label: 'Cycles', value: String(l0.cycles.length), tone: 'warning', lens: 'cycles' },
    { label: 'Same layer · down', value: `${count('level')} · ${count('down')}`, tone: 'ink' },
  ];
}

function workspace(l0: Level0): Inspection {
  return {
    eyebrow: 'Workspace',
    title: `${n(l0.budget.modules)} modules`,
    sub: 'Nothing selected. The map draws modules, not symbols; open one to see its files.',
    stats: workspaceStats(l0).slice(1).map(({ label, value, tone }) => ({ label, value, tone })),
    listLabel: '',
    list: [],
    open: null,
  };
}

function empty(): Inspection {
  return { eyebrow: 'Workspace', title: '', sub: '', stats: [], listLabel: '', list: [], open: null };
}
