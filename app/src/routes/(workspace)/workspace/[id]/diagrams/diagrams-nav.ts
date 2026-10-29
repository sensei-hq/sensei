import type { Lens } from '@sensei/codemap';
import { KANJI_ICONS } from '$lib/glyph-vocabulary';

/** The Diagrams list, in the order the sidebar presents it (M16/M17).
 *
 *  ONE LIST: the sidebar's Diagrams subsection renders it, the `[lens]` route
 *  accepts its lenses, and the redirect in `+page.ts` picks its default from it.
 *  The seven Code map lenses come first — one canvas, switched by route without
 *  reloading — and Database · ER last, because it changes subject.
 */
export interface DiagramView {
  /** The route segment under `diagrams/`. */
  id: string;
  label: string;
  /** The sidebar's second line: what to use the view for. */
  hint: string;
  /** The screen's own description — the question this view answers. */
  question: string;
  /** The header's three lines, behind "What it's for" (M17): what the view
   *  shows, what to use it for, and how to read it. */
  shows: string;
  use: string;
  read: string;
  kanji: string;
  /** The Solar Bold Duotone class shown in icon mode (#133) — the vocabulary's
   *  icon for the kanji. */
  icon: string;
  /** The Code map lens this view draws; null for a diagram of its own. */
  lens: Lens | null;
}

const VIEWS: Omit<DiagramView, 'icon'>[] = [
  {
    id: 'structure',
    label: 'Structure',
    hint: 'the lay of the land',
    question: 'Modules, then their files, then their symbols — what touches what',
    shows: 'Modules as blocks inside group bands, one band per module pair; a band\'s width is the number of calls and imports behind it.',
    use: 'Get the lay of the land, and see what a module depends on before you change it.',
    read: 'Upward bands are red. Turn on Colour by direction to tint every band against the layer order: down, same layer, upward, skips a layer.',
    kanji: '構',
    lens: 'structure',
  },
  {
    id: 'wheel',
    label: 'Wheel',
    hint: 'which groups lean on which',
    question: 'Modules on a ring by group, with the dependencies between groups bundled',
    shows: 'Modules on a ring, grouped into arcs by layer or folder; bands bundle between groups.',
    use: 'See at a glance which groups lean on which, and where the heavy coupling is.',
    read: 'A thick strand is heavy coupling between two groups; red strands point upward. Hover a module to light up only its bands.',
    kanji: '輪',
    lens: 'wheel',
  },
  {
    id: 'zones',
    label: 'Zones',
    hint: 'hard to change, or unused',
    question: 'Modules by instability against abstractness — which sit far from the main sequence',
    shows: 'Every module placed by instability (how much it depends on others) against abstractness (how much of it is interfaces and traits).',
    use: 'Find modules that are hard to change because much depends on concrete code, and abstractions nothing uses.',
    read: 'Healthy modules sit near the diagonal. Bottom-left is the zone of pain, top-right the zone of uselessness; the list ranks the furthest.',
    kanji: '域',
    lens: 'zones',
  },
  {
    id: 'matrix',
    label: 'Dependency matrix',
    hint: 'dense areas, cycles',
    question: 'Module by module: rows depend on columns, upward in red',
    shows: 'A module × module grid; rows depend on columns. Blocks on the diagonal are the groups you chose.',
    use: 'Read dense dependencies where the map turns into a hairball, and spot cycles and cross-group leaks.',
    read: 'Red cells point upward and are violations; amber is a layer skip or a cycle. A crowded row or column outside its block is a group leaning on another.',
    kanji: '格',
    lens: 'matrix',
  },
  {
    id: 'cycles',
    label: 'Cycles',
    hint: 'break before refactoring',
    question: 'What depends on itself, and where to cut',
    shows: 'Only the bands that form dependency cycles; the modules involved are tinted.',
    use: 'Break cycles before they block a refactor, a split or a release gate.',
    read: 'Follow the tinted modules; each pair of opposite bands is one cycle. The inspector lists the partner modules.',
    kanji: '環',
    lens: 'cycles',
  },
  {
    id: 'coupling',
    label: 'Hidden coupling',
    hint: 'change together, no edge',
    question: 'Modules that change in the same commits with no dependency between them',
    shows: 'Modules that change together in git history but have no static dependency between them.',
    use: 'Find hidden dependencies: shared assumptions, copy-paste and implicit contracts.',
    read: 'Dashed bands are co-change; hover one for how often the two change in the same commit.',
    kanji: '縁',
    lens: 'coupling',
  },
  {
    id: 'complexity',
    label: 'Complexity',
    hint: 'where to refactor and test',
    question: 'Modules by their share of functions over cyclomatic complexity 10',
    shows: 'Modules tinted by the share of functions over cyclomatic complexity 10.',
    use: 'Pick where to refactor first and where tests matter most.',
    read: 'Red over 20%, amber over 10%. Open a module to see which files carry it.',
    kanji: '繁',
    lens: 'complexity',
  },
  {
    id: 'schema',
    label: 'Database · ER',
    hint: 'tables, columns and keys',
    question: 'Which tables exist, and how their keys connect them',
    shows: 'Tables with their columns and keys, and the views and routines that read them.',
    use: 'See what a change to a table reaches before you make it.',
    read: 'Lines are foreign keys. Select a table to list its columns, its keys and what depends on it.',
    kanji: '表',
    lens: null,
  },
];

export const DIAGRAM_VIEWS: DiagramView[] = VIEWS.map((v) => ({ ...v, icon: KANJI_ICONS[v.kanji] }));

/** The view a `diagrams/` URL opens on: Structure, modules first (M17). */
export const DEFAULT_VIEW = DIAGRAM_VIEWS[0].id;

/** The Code map lens a route segment draws, or null when it is not one. */
export function lensOf(id: string): Lens | null {
  return DIAGRAM_VIEWS.find((v) => v.id === id)?.lens ?? null;
}

/** Which view a pathname is on. Returns null off the section entirely, so a
 *  caller cannot read "no view" as "the first view". */
export function viewOf(pathname: string): string | null {
  const found = DIAGRAM_VIEWS.find((v) => pathname.endsWith(`/diagrams/${v.id}`));
  return found?.id ?? null;
}
