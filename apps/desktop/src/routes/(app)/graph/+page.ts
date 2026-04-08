import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const data = await fetch('/api/graph').then(r => r.json());
  return data;
};
