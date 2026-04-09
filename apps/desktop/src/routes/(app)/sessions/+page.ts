import type { PageLoad } from './$types';

const SENSEI_API = 'http://localhost:7744';

export const load: PageLoad = async ({ fetch }) => {
  try {
    const res = await fetch(`${SENSEI_API}/api/sessions`);
    if (res.ok) return res.json();
  } catch { /* sensei serve not running */ }
  return { stats: null, sessions: [], toolUsage: [], benchmarkPairs: [] };
};
