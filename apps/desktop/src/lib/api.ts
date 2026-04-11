import type { ServerProject, GraphData, SessionData } from './types.js';

/** Create a typed API client for the sensei backend. */
export function senseiApi(port: number) {
  const base = `http://127.0.0.1:${port}`;

  async function json<T>(url: string, fallback: T): Promise<T> {
    try {
      const res = await fetch(url);
      return res.ok ? await res.json() as T : fallback;
    } catch {
      return fallback;
    }
  }

  return {
    getHealth: () => json<Record<string, unknown>>(`${base}/health`, {}),

    getProjects: () => json<ServerProject[]>(`${base}/api/projects`, []),

    getGraph: (repoId: string, repoPath: string) =>
      json<GraphData>(
        `${base}/api/graph?repoId=${encodeURIComponent(repoId)}&repoPath=${encodeURIComponent(repoPath)}`,
        { summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, communities: [], godNodes: [], rationale: [] },
      ),

    getSessions: () =>
      json<SessionData>(`${base}/api/sessions`, { stats: null, sessions: [], toolUsage: [], benchmarkPairs: [] }),

    getDrift: (repoPath: string) =>
      json<{ drifted: unknown[]; summary: string }>(`${base}/api/drift?repoPath=${encodeURIComponent(repoPath)}`, { drifted: [], summary: '' }),

    registerProject: async (repoId: string, name: string, path: string) => {
      try {
        await fetch(`${base}/api/projects`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ repoId, name, path }),
        });
      } catch { /* non-fatal */ }
    },

    indexRepo: async (repoId: string, repoPath: string, force = false) => {
      return fetch(`${base}/api/index`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ repoId, repoPath, force }),
      });
    },
  };
}
