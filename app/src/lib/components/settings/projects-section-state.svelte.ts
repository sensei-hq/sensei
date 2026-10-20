// Settings · Projects — what the screen lists, and saving a repository's role (#271).
//
// A project's REPOSITORIES are its git checkouts and subtrees. `GET /api/projects`
// returns the project's whole folder tree, and the screen used to list every
// entry: a monorepo's workspace modules and plain folders read as "19 repos".
//
// A role chosen here is saved at once, through the daemon. It used to change
// only local wizard state, which the Settings screen never commits, so it was
// gone on reload. A failed save is rolled back and reported, not left on screen.
//
// A failed load is an error state. `api.listProjects` turned any failure into
// `[]`, which read as "no projects" — a fabricated empty.
import type { ApiResult } from '$lib/api.js';
import type { DaemonProject, DaemonProjectFolder } from '$lib/setup/contracts.js';
import { ROLES } from '$lib/setup/types.js';
import type { GlyphMode } from '$lib/glyphs';

/** Folder kinds that are a repository of their own. */
const REPOSITORY_KINDS = new Set(['git', 'subtree']);

/** Pure: a project's repositories — its git checkouts and subtrees. */
export function repositoriesOf(p: DaemonProject): DaemonProjectFolder[] {
  return p.folders.filter((f) => REPOSITORY_KINDS.has(f.kind));
}

const inside = (dir: string, path: string) => path.startsWith(`${dir}/`);

/** Pure: the roles the scan inferred for the packages inside a repository,
 *  distinct and in picker order. A package belongs to the nearest repository
 *  that holds it, so a subtree's packages are not also its parent's. */
export function derivedRoles(p: DaemonProject, repoId: string): string[] {
  const repos = repositoriesOf(p);
  const repo = repos.find((r) => r.id === repoId);
  if (!repo) return [];
  const nearest = (f: DaemonProjectFolder) =>
    repos
      .filter((r) => inside(r.abs_path, f.abs_path))
      .reduce<DaemonProjectFolder | null>(
        (best, r) => (best && best.abs_path.length >= r.abs_path.length ? best : r),
        null,
      );
  const found = new Set(
    p.folders
      .filter((f) => f.kind === 'module' && f.role && nearest(f)?.id === repo.id)
      .map((f) => f.role as string),
  );
  return ROLES.map((r) => r.id).filter((id) => found.has(id));
}

/** Pure: projects with at least one repository on this machine, and those
 *  without (their checkouts were removed, or never scanned here). */
export function partitionProjects(projects: DaemonProject[]): {
  placed: DaemonProject[];
  unplaced: DaemonProject[];
} {
  const placed: DaemonProject[] = [];
  const unplaced: DaemonProject[] = [];
  for (const p of projects) (repositoriesOf(p).length > 0 ? placed : unplaced).push(p);
  return { placed, unplaced };
}

/** Pure: does the project match the search — its name, or a repository's name
 *  or path — ignoring case? A blank query matches everything. */
export function matchesSearch(p: DaemonProject, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q === '') return true;
  if (p.name.toLowerCase().includes(q)) return true;
  return repositoriesOf(p).some(
    (r) => r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q),
  );
}

/** The two calls the screen needs, injected so the state is testable. */
export interface ProjectsApi {
  tryListProjects: () => Promise<ApiResult<DaemonProject[]>>;
  tryUpdateFolder: (id: string, patch: { role?: string | null }) => Promise<ApiResult<void>>;
  tryUpdateProject: (id: string, patch: object) => Promise<ApiResult<void>>;
}

export class ProjectsSectionState {
  projects = $state<DaemonProject[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);
  /** The last failed role save, shown until the next attempt. */
  roleError = $state<string | null>(null);
  /** The last failed rename, shown until the next attempt. */
  renameError = $state<string | null>(null);
  query = $state('');

  readonly placed = $derived(
    partitionProjects(this.projects).placed.filter((p) => matchesSearch(p, this.query)),
  );
  readonly unplaced = $derived(
    partitionProjects(this.projects).unplaced.filter((p) => matchesSearch(p, this.query)),
  );

  async load(api: ProjectsApi): Promise<void> {
    this.loading = true;
    const res = await api.tryListProjects();
    if (res.ok) {
      this.projects = res.data.map((p) => ({ ...p, folders: p.folders ?? [] }));
      this.error = null;
    } else {
      this.error = `Could not load projects: ${res.error.message || `HTTP ${res.error.status}`}`;
    }
    this.loading = false;
  }

  /** Rename a project, saved at once; the old name comes back if the daemon
   *  refuses it. A blank or unchanged name is not sent. */
  async rename(api: ProjectsApi, projectId: string, name: string): Promise<void> {
    const target = this.projects.find((p) => p.id === projectId);
    const next = name.trim();
    if (!target || next === '' || next === target.name) return;
    const previous = target.name;
    target.name = next;
    this.renameError = null;
    const res = await api.tryUpdateProject(projectId, { name: next });
    if (!res.ok) {
      target.name = previous;
      this.renameError = `Could not rename ${previous}: ${res.error.message || `HTTP ${res.error.status}`}`;
    }
  }

  /** Save a repository's role. The screen shows the new role at once and puts
   *  the old one back if the daemon refuses it. `''` clears the role. */
  async setRole(api: ProjectsApi, folderId: string, role: string): Promise<void> {
    const target = this.projects.flatMap((p) => p.folders).find((f) => f.id === folderId);
    if (!target) return;
    const previous = target.role;
    target.role = role === '' ? null : role;
    this.roleError = null;
    const res = await api.tryUpdateFolder(folderId, { role: target.role });
    if (!res.ok) {
      target.role = previous;
      this.roleError = `Could not save the role for ${target.name}: ${res.error.message || `HTTP ${res.error.status}`}`;
    }
  }
}

/** The role picker's items for a glyph mode (#133). */
export function roleOptions(mode: GlyphMode): { label: string; value: string; icon?: string }[] {
  return [
    { label: 'Unassigned', value: '' },
    ...ROLES.map((r) =>
      mode === 'icon'
        ? { label: r.label, value: r.id, icon: r.icon }
        : { label: `${r.kanji} ${r.label}`, value: r.id },
    ),
  ];
}
