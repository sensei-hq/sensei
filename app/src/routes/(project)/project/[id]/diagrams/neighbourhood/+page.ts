import type { PageLoad } from './$types.js';

/** The page owns its own fetching, for the reason Structure does: the focus and
 *  depth controls re-query the daemon, and a loader runs once per navigation —
 *  so a loader-held fetch would make the first render a loader concern and every
 *  later one a component concern, two code paths for one set of states.
 *
 *  `focus` comes from the URL so a neighbourhood can be linked to and survives
 *  a reload. It is read here, once, and the page owns it from then on. */
export const load: PageLoad = async ({ parent, url }) => {
  const { project, projectId } = await parent();
  return { project, projectId, focus: url.searchParams.get('focus') };
};
