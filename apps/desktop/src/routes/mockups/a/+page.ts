import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const res = await fetch('/api/mockups/a');
  const data = await res.json();
  return data;
};
