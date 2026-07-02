import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/**
 * The project detail is a sidebar-driven shell — `/projects/[id]` itself has
 * no content, so kick straight to the Overview section. Downstream sections
 * live at `/projects/[id]/{overview, sessions, memories, ...}`.
 */
export const load: PageLoad = ({ params }) => {
  throw redirect(307, `/projects/${params.id}/overview`);
};
