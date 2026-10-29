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
    kanji: '構',
    lens: 'structure',
  },
  {
    id: 'wheel',
    label: 'Wheel',
    hint: 'which groups lean on which',
    question: 'Modules on a ring by group, with the dependencies between groups bundled',
    kanji: '輪',
    lens: 'wheel',
  },
  {
    id: 'zones',
    label: 'Zones',
    hint: 'hard to change, or unused',
    question: 'Modules by instability against abstractness — which sit far from the main sequence',
    kanji: '域',
    lens: 'zones',
  },
  {
    id: 'matrix',
    label: 'Dependency matrix',
    hint: 'dense areas, cycles',
    question: 'Module by module: rows depend on columns, upward in red',
    kanji: '格',
    lens: 'matrix',
  },
  {
    id: 'cycles',
    label: 'Cycles',
    hint: 'break before refactoring',
    question: 'What depends on itself, and where to cut',
    kanji: '環',
    lens: 'cycles',
  },
  {
    id: 'coupling',
    label: 'Hidden coupling',
    hint: 'change together, no edge',
    question: 'Modules that change in the same commits with no dependency between them',
    kanji: '縁',
    lens: 'coupling',
  },
  {
    id: 'complexity',
    label: 'Complexity',
    hint: 'where to refactor and test',
    question: 'Modules by their share of functions over cyclomatic complexity 10',
    kanji: '繁',
    lens: 'complexity',
  },
  {
    id: 'schema',
    label: 'Database · ER',
    hint: 'tables, columns and keys',
    question: 'Which tables exist, and how their keys connect them',
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
