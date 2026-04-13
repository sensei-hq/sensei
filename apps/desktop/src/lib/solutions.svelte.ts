import type { Solution, SolutionRepo, SolutionCategory, RepoRole, ScannedRepo } from './types.js';

// ─── Reactive state ──────────────────────────────────────────────────────────

let _solutions = $state<Solution[]>([]);
let _activeSolutionId = $state<string | null>(null);
let _loaded = $state(false);

export function getSolutions(): Solution[] { return _solutions; }
export function getActiveSolutionId(): string | null { return _activeSolutionId; }
export function isSolutionsLoaded(): boolean { return _loaded; }
export function markLoaded() { _loaded = true; }

export async function setActiveSolutionId(id: string | null) {
  _activeSolutionId = id;
  const { setConfigValue, getPort } = await import('./appstate.svelte.js');
  if (id) await setConfigValue('active_solution', id);
}

// ─── Persistence (daemon only) ──────────────────────────────────────────────

export async function loadSolutions() {
  await syncFromServer();
  // Load active solution from daemon config
  const { getActiveSolutionId: getFromConfig } = await import('./appstate.svelte.js');
  _activeSolutionId = getFromConfig();
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

export function createSolution(
  name: string,
  repos: SolutionRepo[],
  opts?: { description?: string; client?: string; category?: SolutionCategory }
): Solution {
  const solution: Solution = {
    id: crypto.randomUUID(),
    name,
    description: opts?.description,
    client: opts?.client,
    category: opts?.category ?? 'active',
    repos,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  _solutions = [..._solutions, solution];
  pushSolutionToServer(solution);
  return solution;
}

export function updateSolution(id: string, patch: Partial<Omit<Solution, 'id' | 'createdAt'>>) {
  _solutions = _solutions.map(s =>
    s.id === id ? { ...s, ...patch, updatedAt: new Date().toISOString() } : s
  );
}

export function deleteSolution(id: string) {
  _solutions = _solutions.filter(s => s.id !== id);
  if (_activeSolutionId === id) setActiveSolutionId(_solutions[0]?.id ?? null);
}

export function addRepoToSolution(solutionId: string, repo: SolutionRepo) {
  _solutions = _solutions.map(s => {
    if (s.id !== solutionId) return s;
    if (s.repos.some(r => r.path === repo.path)) return s;
    return { ...s, repos: [...s.repos, repo], updatedAt: new Date().toISOString() };
  });
}

export function removeRepoFromSolution(solutionId: string, path: string) {
  _solutions = _solutions.map(s => {
    if (s.id !== solutionId) return s;
    return { ...s, repos: s.repos.filter(r => r.path !== path), updatedAt: new Date().toISOString() };
  });
}

// ─── Lookups ─────────────────────────────────────────────────────────────────

export function getSolutionById(id: string): Solution | undefined {
  return _solutions.find(s => s.id === id);
}

export function getSolutionForRepo(path: string): Solution | undefined {
  return _solutions.find(s => s.repos.some(r => r.path === path));
}

export function getSolutionsByCategory(category: SolutionCategory): Solution[] {
  return _solutions.filter(s => s.category === category);
}

export function getStandaloneLibraries(): Solution[] {
  return [];
}

// ─── Role inference ──────────────────────────────────────────────────────────

export function inferRepoRole(repo: ScannedRepo): RepoRole {
  const cats = repo.categories ?? [];
  const stack = repo.tech_stack ?? [];
  if (cats.includes('library')) return 'library';
  if (cats.includes('tool')) return 'shared';
  const stackLower = stack.map(s => s.toLowerCase());
  if (stackLower.some(s => ['react', 'vue', 'svelte', 'sveltekit', 'nextjs', 'nuxt', 'angular'].includes(s))) return 'frontend';
  if (stackLower.some(s => ['react-native', 'flutter', 'swift', 'kotlin', 'ios', 'android'].includes(s))) return 'mobile';
  if (stackLower.some(s => ['express', 'fastify', 'hono', 'django', 'flask', 'spring', 'actix', 'axum', 'gin'].includes(s))) return 'backend';
  if (stackLower.some(s => ['terraform', 'pulumi', 'kubernetes', 'docker', 'helm'].includes(s))) return 'infra';
  if (cats.includes('app')) return 'backend';
  return 'unknown';
}

// ─── Server sync ─────────────────────────────────────────────────────────────

async function pushSolutionToServer(solution: Solution) {
  const port = getPort();
  const { senseiApi } = await import('./api.js');
  const api = senseiApi(port);
  try {
    await api.createSolution({
      id: solution.id, name: solution.name, description: solution.description,
      client: solution.client, category: solution.category,
      repos: solution.repos.map(r => ({ repo_id: r.repoId, role: r.role, label: r.label })),
      tags: [],
    });
  } catch { /* non-fatal */ }
}

async function syncFromServer() {
  const port = getPort();
  const { senseiApi } = await import('./api.js');
  const api = senseiApi(port);
  try {
    const serverSolutions = await api.listSolutions();
    const seen = new Set<string>();
    const clean: Solution[] = [];
    for (const s of serverSolutions) {
      const mapped = mapServerSolution(s);
      if (!seen.has(mapped.id)) {
        seen.add(mapped.id);
        clean.push(mapped);
      }
    }
    _solutions = clean;
  } catch { /* server may not be running */ }
}

function mapServerSolution(s: any): Solution {
  return {
    id: s.id, name: s.name, description: s.description, client: s.client,
    category: s.category ?? 'active',
    repos: (s.repos ?? []).map((r: any) => ({
      repoId: r.repo_id ?? r.repoId, path: r.path ?? '', role: r.role ?? 'unknown', label: r.label,
    })),
    createdAt: s.created_at ?? s.createdAt ?? new Date().toISOString(),
    updatedAt: s.updated_at ?? s.updatedAt ?? new Date().toISOString(),
  };
}

function getPort(): number {
  try {
    return parseInt(typeof localStorage !== 'undefined' ? (localStorage.getItem('sensei:port') ?? '7744') : '7744', 10);
  } catch { return 7744; }
}

// ─── Reset ───────────────────────────────────────────────────────────────────

export function clearAllSolutions() {
  _solutions = [];
  _activeSolutionId = null;
}
