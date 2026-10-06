import type { PageLoad } from './$types.js';

/** The page owns its own fetching, for the reason Structure does: the grain
 *  and edge-kind controls re-query the daemon, and a loader runs once per
 *  navigation — so a loader-held fetch would make the first render a loader
 *  concern and every later one a component concern, two code paths for one set
 *  of three states. */
export const load: PageLoad = async ({ parent }) => {
  const { project, projectId } = await parent();
  return { project, projectId };
};
