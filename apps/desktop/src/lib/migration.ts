import type { ScannedRepo, SolutionRepo } from './types.js';
import { createSolution, getSolutions, loadSolutions, setActiveSolutionId, inferRepoRole } from './solutions.svelte.js';

const MIGRATION_FLAG = 'sensei:migration_v1';

/**
 * One-time migration from sensei:projects_raw → sensei:solutions.
 * Groups repos by variant_group, creates solutions.
 * Never deletes projects_raw (server sync still reads it).
 */
export function migrate(): void {
  if (localStorage.getItem(MIGRATION_FLAG)) return;
  if (localStorage.getItem('sensei:solutions')) {
    localStorage.setItem(MIGRATION_FLAG, '1');
    return;
  }

  let repos: ScannedRepo[] = [];
  try {
    const raw = localStorage.getItem('sensei:projects_raw');
    if (raw) repos = JSON.parse(raw) as ScannedRepo[];
  } catch { /* empty */ }

  if (repos.length === 0) {
    localStorage.setItem(MIGRATION_FLAG, '1');
    return;
  }

  // Apply variant overrides
  let overrides: Record<string, string | null> = {};
  try {
    const raw = localStorage.getItem('sensei:variant_overrides');
    if (raw) overrides = JSON.parse(raw) as Record<string, string | null>;
  } catch { /* empty */ }

  // Group by effective variant_group
  const groups = new Map<string, ScannedRepo[]>();
  const ungrouped: ScannedRepo[] = [];

  for (const repo of repos) {
    if (repo.duplicate_of) continue; // skip duplicates
    const effectiveGroup = overrides[repo.path] ?? overrides[repo.repoId ?? ''] ?? repo.variant_group;
    if (effectiveGroup) {
      const list = groups.get(effectiveGroup) ?? [];
      list.push(repo);
      groups.set(effectiveGroup, list);
    } else {
      ungrouped.push(repo);
    }
  }

  // Create solutions from groups (only groups with 2+ repos)
  for (const [groupName, members] of groups) {
    if (members.length < 2) {
      ungrouped.push(...members);
      continue;
    }
    const solutionRepos: SolutionRepo[] = members.map(r => ({
      repoId: r.repoId ?? r.path.replace(/^\//, ''),
      path: r.path,
      role: inferRepoRole(r),
      label: r.name,
    }));

    const allIdeas = members.every(r => r.categories?.includes('idea'));
    createSolution(
      deriveSolutionName(groupName, members),
      solutionRepos,
      {
        category: allIdeas ? 'idea' : 'active',
        client: members.find(r => r.client)?.client,
      },
    );
  }

  // Create solutions from ungrouped repos
  for (const repo of ungrouped) {
    const isIdea = repo.categories?.includes('idea');
    const isSide = (repo.last_commit_days ?? 0) > 90 || repo.status === 'stale' || repo.status === 'archived';

    createSolution(repo.name, [{
      repoId: repo.repoId ?? repo.path.replace(/^\//, ''),
      path: repo.path,
      role: inferRepoRole(repo),
      label: repo.name,
    }], {
      category: isIdea ? 'idea' : isSide ? 'side' : 'active',
      client: repo.client,
    });
  }

  // Set first active solution as default
  const solutions = getSolutions();
  const firstActive = solutions.find(s => s.category === 'active');
  if (firstActive) setActiveSolutionId(firstActive.id);

  localStorage.setItem(MIGRATION_FLAG, '1');
}

/** Derive a clean solution name from the variant group stem and its members. */
function deriveSolutionName(groupStem: string, members: ScannedRepo[]): string {
  // Use the group stem, cleaned up
  const name = groupStem
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase())
    .trim();
  return name || members[0]?.name || 'Unnamed Solution';
}
