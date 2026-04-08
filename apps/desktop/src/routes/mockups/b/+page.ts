import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const res = await fetch('/api/mockups/b');
  const data = await res.json();
  return data;
};
