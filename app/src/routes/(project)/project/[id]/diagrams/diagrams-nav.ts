/** The views inside Diagrams, in the order the section presents them.
 *
 *  ONE LIST, because the sub-nav renders it and the redirect in `+page.ts`
 *  picks its default from the same place. Two copies is how a view appears in
 *  the nav months before anything routes to it.
 *
 *  Order is coarsening-then-narrowing, which is how a reader arrives: Structure
 *  shows what there is, Layers ranks it, Cycles isolates the part of it that
 *  does not rank.
 */
export interface DiagramView {
  /** The route segment under `diagrams/`. */
  id: string;
  label: string;
  /** One line, shown as the screen's own description — the question this view
   *  answers, so a reader can tell two pictures of one graph apart. */
  question: string;
  kanji: string;
}

export const DIAGRAM_VIEWS: DiagramView[] = [
  {
    id: 'structure',
    label: 'Structure',
    question: 'What is here, and what touches what',
    kanji: '構',
  },
  {
    id: 'layers',
    label: 'Layers',
    question: 'Do these calls flow downward',
    kanji: '層',
  },
  {
    id: 'cycles',
    label: 'Cycles',
    question: 'What depends on itself, and where to cut',
    kanji: '環',
  },
];

/** The view a `diagrams/` URL opens on. Structure, because it is the only one
 *  that assumes nothing about the layering having ranked. */
export const DEFAULT_VIEW = DIAGRAM_VIEWS[0].id;

/** Which tab a pathname is on. Returns null off the section entirely, so a
 *  caller cannot read "no tab" as "the first tab". */
export function viewOf(pathname: string): string | null {
  const found = DIAGRAM_VIEWS.find((v) => pathname.endsWith(`/diagrams/${v.id}`));
  return found?.id ?? null;
}
