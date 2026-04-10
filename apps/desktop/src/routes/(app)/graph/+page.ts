import type { PageLoad } from './$types';

const SENSEI_API = 'http://localhost:7744';

export const load: PageLoad = async ({ fetch }) => {
  try {
    const res = await fetch(`${SENSEI_API}/api/projects`);
    if (res.ok) {
      const projects = await res.json() as { repoId: string; name: string; path: string; indexedAt?: string }[];
      return { projects, summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, communities: [], godNodes: [], rationale: [] };
    }
  } catch { /* server offline */ }
  return { projects: [], summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, communities: [], godNodes: [], rationale: [] };
};
