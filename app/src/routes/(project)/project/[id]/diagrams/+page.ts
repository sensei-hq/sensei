import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';
import { DEFAULT_VIEW } from './diagrams-nav.js';

/** Structure is the section's default view — the mockup opens on it
 *  (`startCalls: "structure"`), and it is the only one that assumes nothing
 *  about the layering having ranked.
 *
 *  The segment comes from `DIAGRAM_VIEWS` rather than a literal, so the tab
 *  this lands on is the tab the sub-nav draws first. */
export const load: PageLoad = ({ params }) => {
  redirect(307, `/project/${params.id}/diagrams/${DEFAULT_VIEW}`);
};
