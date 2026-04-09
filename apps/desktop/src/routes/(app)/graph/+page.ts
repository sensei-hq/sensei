import type { PageLoad } from './$types';

const SENSEI_API = 'http://localhost:7744';

export const load: PageLoad = async ({ fetch }) => {
  try {
    const res = await fetch(`${SENSEI_API}/api/graph`);
    if (res.ok) return res.json();
  } catch { /* sensei serve not running */ }
  // Fallback — empty structure
  return { summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, projects: [], communities: [], godNodes: [], rationale: [] };
};
