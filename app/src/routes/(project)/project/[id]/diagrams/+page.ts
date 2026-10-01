import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';

/** Structure is the section's default view — the mockup opens on it
 *  (`startCalls: "structure"`), and it is the only one that needs no new
 *  indexing and no new rokkit component. */
export const load: PageLoad = ({ params }) => {
  redirect(307, `/project/${params.id}/diagrams/structure`);
};
