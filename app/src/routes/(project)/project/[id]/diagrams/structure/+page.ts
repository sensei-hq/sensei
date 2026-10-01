import type { PageLoad } from './$types.js';

/** The page owns its own fetching.
 *
 *  The level and edge-kind controls re-query the daemon, and a loader can only
 *  run once per navigation — so putting the fetch here would make the FIRST
 *  render a loader concern and every later one a component concern, with two
 *  code paths for the same three states. One owner, in the component.
 */
export const load: PageLoad = async ({ parent }) => {
  const { project, projectId } = await parent();
  return { project, projectId };
};
