import type { Solution, SolutionRepo, SolutionCategory, RepoRole, ScannedRepo } from './types.js';

const SOLUTIONS_KEY = 'sensei:solutions';
const ACTIVE_KEY = 'sensei:active_solution';

// ─── Reactive state ──────────────────────────────────────────────────────────

let _solutions = $state<Solution[]>([]);
let _activeSolutionId = $state<string | null>(null);

export function getSolutions(): Solution[] { return _solutions; }
export function getActiveSolutionId(): string | null { return _activeSolutionId; }

export function setActiveSolutionId(id: string | null) {
  _activeSolutionId = id;
  if (id) localStorage.setItem(ACTIVE_KEY, id);
  else localStorage.removeItem(ACTIVE_KEY);
}

// ─── Persistence ─────────────────────────────────────────────────────────────

export function loadSolutions() {
  try {
    const raw = localStorage.getItem(SOLUTIONS_KEY);
    if (raw) _solutions = JSON.parse(raw) as Solution[];
  } catch { _solutions = []; }
  _activeSolutionId = localStorage.getItem(ACTIVE_KEY);
}

export function saveSolutions() {
  localStorage.setItem(SOLUTIONS_KEY, JSON.stringify(_solutions));
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
  saveSolutions();
  return solution;
}

export function updateSolution(id: string, patch: Partial<Omit<Solution, 'id' | 'createdAt'>>) {
  _solutions = _solutions.map(s =>
    s.id === id ? { ...s, ...patch, updatedAt: new Date().toISOString() } : s
  );
  saveSolutions();
}

export function deleteSolution(id: string) {
  _solutions = _solutions.filter(s => s.id !== id);
  if (_activeSolutionId === id) setActiveSolutionId(_solutions[0]?.id ?? null);
  saveSolutions();
}

export function addRepoToSolution(solutionId: string, repo: SolutionRepo) {
  _solutions = _solutions.map(s => {
    if (s.id !== solutionId) return s;
    if (s.repos.some(r => r.path === repo.path)) return s;
    return { ...s, repos: [...s.repos, repo], updatedAt: new Date().toISOString() };
  });
  saveSolutions();
}

export function removeRepoFromSolution(solutionId: string, path: string) {
  _solutions = _solutions.map(s => {
    if (s.id !== solutionId) return s;
    return { ...s, repos: s.repos.filter(r => r.path !== path), updatedAt: new Date().toISOString() };
  });
  saveSolutions();
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

/** Libraries that appear in 2+ solutions → standalone status */
export function getStandaloneLibraries(): Solution[] {
  const libPaths = new Map<string, string[]>(); // path → solutionIds
  for (const s of _solutions) {
    for (const r of s.repos) {
      if (r.role === 'library') {
        const ids = libPaths.get(r.path) ?? [];
        ids.push(s.id);
        libPaths.set(r.path, ids);
      }
    }
  }
  // Return solutions that ARE a standalone library (single-repo solution with the lib)
  return _solutions.filter(s =>
    s.repos.length === 1 &&
    s.repos[0].role === 'library' &&
    (libPaths.get(s.repos[0].path)?.length ?? 0) >= 2
  );
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

// ─── Reset ───────────────────────────────────────────────────────────────────

export function clearAllSolutions() {
  _solutions = [];
  _activeSolutionId = null;
  localStorage.removeItem(SOLUTIONS_KEY);
  localStorage.removeItem(ACTIVE_KEY);
  localStorage.removeItem('sensei:migration_v1');
}
