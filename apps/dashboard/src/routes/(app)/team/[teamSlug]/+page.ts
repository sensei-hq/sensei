import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
  const res = await fetch(`/api/mockups/team/${params.teamSlug}`);
  return await res.json();
};
