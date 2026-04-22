import type { Project, ProjectRepo, ProjectCategory, RepoRole, ScannedRepo } from './types.js';

// ─── Reactive state ──────────────────────────────────────────────────────────

let _projects = $state<Project[]>([]);
let _activeProjectId = $state<string | null>(null);
let _loaded = $state(false);

export function getProjects(): Project[] { return _projects; }
export function getActiveProjectId(): string | null { return _activeProjectId; }
export function isProjectsLoaded(): boolean { return _loaded; }
export function markLoaded() { _loaded = true; }

export async function setActiveProjectId(id: string | null) {
  _activeProjectId = id;
  const { setConfigValue, getPort } = await import('./appstate.svelte.js');
  if (id) await setConfigValue('active_project', id);
}

// ─── Persistence (daemon only) ──────────────────────────────────────────────

export async function loadProjects() {
  await syncFromServer();
  const { getActiveProjectId: getFromConfig } = await import('./appstate.svelte.js');
  _activeProjectId = getFromConfig();
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

export function createProject(
  name: string,
  repos: ProjectRepo[],
  opts?: { description?: string; client?: string; category?: ProjectCategory }
): Project {
  const project: Project = {
    id: crypto.randomUUID(),
    name,
    description: opts?.description,
    client: opts?.client,
    category: opts?.category ?? 'active',
    repos,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  _projects = [..._projects, project];
  pushProjectToServer(project);
  return project;
}

export function updateProject(id: string, patch: Partial<Omit<Project, 'id' | 'createdAt'>>) {
  _projects = _projects.map(p =>
    p.id === id ? { ...p, ...patch, updatedAt: new Date().toISOString() } : p
  );
}

export function deleteProject(id: string) {
  _projects = _projects.filter(p => p.id !== id);
  if (_activeProjectId === id) setActiveProjectId(_projects[0]?.id ?? null);
}

export function addRepoToProject(projectId: string, repo: ProjectRepo) {
  _projects = _projects.map(p => {
    if (p.id !== projectId) return p;
    if (p.repos.some(r => r.path === repo.path)) return p;
    return { ...p, repos: [...p.repos, repo], updatedAt: new Date().toISOString() };
  });
}

export function removeRepoFromProject(projectId: string, path: string) {
  _projects = _projects.map(p => {
    if (p.id !== projectId) return p;
    return { ...p, repos: p.repos.filter(r => r.path !== path), updatedAt: new Date().toISOString() };
  });
}

// ─── Lookups ─────────────────────────────────────────────────────────────────

export function getProjectById(id: string): Project | undefined {
  return _projects.find(p => p.id === id);
}

export function getProjectForRepo(path: string): Project | undefined {
  return _projects.find(p => p.repos.some(r => r.path === path));
}

export function getProjectsByCategory(category: ProjectCategory): Project[] {
  return _projects.filter(p => p.category === category);
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

async function pushProjectToServer(project: Project) {
  const port = getPort();
  const { senseiApi } = await import('./api.js');
  const api = senseiApi(port);
  try {
    await api.createProject({
      id: project.id, name: project.name, description: project.description,
      client: project.client, category: project.category,
      repos: project.repos.map(r => ({ repo_id: r.repoId, role: r.role, label: r.label })),
      tags: [],
    });
  } catch { /* non-fatal */ }
}

async function syncFromServer() {
  const port = getPort();
  const { senseiApi } = await import('./api.js');
  const api = senseiApi(port);
  try {
    const serverProjects = await api.listProjects();
    const seen = new Set<string>();
    const clean: Project[] = [];
    for (const p of serverProjects) {
      const mapped = mapServerProject(p);
      if (!seen.has(mapped.id)) {
        seen.add(mapped.id);
        clean.push(mapped);
      }
    }
    _projects = clean;
  } catch { /* server may not be running */ }
}

function mapServerProject(s: any): Project {
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

export function clearAllProjects() {
  _projects = [];
  _activeProjectId = null;
}
