import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const ideas = await fetch('/api/ideas').then(r => r.json());
  return { ideas };
};
