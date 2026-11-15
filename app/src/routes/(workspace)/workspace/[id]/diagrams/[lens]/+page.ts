import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';
import { DIAGRAM_VIEWS, lensOf } from '../diagrams-nav.js';

/** One route for every Code map lens (M17): each is the same canvas, so the
 *  page only names the lens; the map's state lives in the diagrams layout and
 *  keeps the level and selection across lenses. A segment that is no lens is a
 *  404, never the default lens. */
export const load: PageLoad = ({ params }) => {
  const lens = lensOf(params.lens);
  if (!lens) error(404, `No diagram "${params.lens}"`);
  const view = DIAGRAM_VIEWS.find((v) => v.id === params.lens)!;
  return { workspaceId: params.id, lens, view };
};
