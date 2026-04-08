import type { PageLoad } from './$types';

type ScannedRepo = {
  name: string; path: string; remote: string | null; description: string | null;
  categories: string[]; status: string; last_commit_days: number | null;
  tech_stack: string[]; commit_count: number;
  duplicate_of: string | null; variant_group: string | null;
};

function activityLabel(days: number | null): string {
  if (days === null) return 'Unknown';
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7)  return `${days} days ago`;
  if (days < 30) return `${Math.floor(days / 7)} weeks ago`;
  if (days < 365) return `${Math.floor(days / 30)} months ago`;
  return `${Math.floor(days / 365)} years ago`;
}

const MATURITY: Record<string, number> = {
  active: 3, recent: 2, stale: 1, archived: 1, abandoned: 0, unknown: 1,
};
const PHASE: Record<string, string> = {
  active: 'Implementation', recent: 'Implementation',
  stale: 'Review', archived: 'Review', abandoned: 'Exploration', unknown: 'Exploration',
};

function repoToProject(repo: ScannedRepo, i: number) {
  return {
    id: `scanned-${i}-${repo.path}`,
    kind: (repo.categories ?? []).includes('idea') && !(repo.categories ?? []).includes('app') ? 'idea' : 'repo',
    name: repo.name,
    path: repo.path,
    description: repo.description ?? repo.name,
    language: repo.tech_stack[0] ?? '',
    maturity: MATURITY[repo.status] ?? 1,
    activePhase: PHASE[repo.status] ?? 'Exploration',
    lastActivity: activityLabel(repo.last_commit_days),
    sessionCount: 0, cardCount: 0, symbolCount: 0, ftrScore: 0,
    phases: [
      { name: 'Requirements',   done: false, active: false, cardCount: 0 },
      { name: 'Analysis',       done: false, active: false, cardCount: 0 },
      { name: 'Design',         done: false, active: false, cardCount: 0 },
      { name: 'Implementation', done: false, active: false, cardCount: 0 },
      { name: 'Review',         done: false, active: false, cardCount: 0 },
    ],
    cards: [], sessions: [], godNodes: [], communities: [], rationale: [],
    // Scan metadata — present only for imported repos
    scanStatus: repo.status,
    tech_stack: repo.tech_stack,
    commit_count: repo.commit_count,
    category: repo.categories?.[0] ?? 'unknown',
    categories: repo.categories ?? [],
    variant_group: repo.variant_group,
    remote: repo.remote,
  };
}

export const load: PageLoad = async ({ fetch }) => {
  const raw = localStorage.getItem('sensei:projects_raw');
  if (raw) {
    try {
      const scanned: ScannedRepo[] = JSON.parse(raw);
      if (scanned.length > 0) {
        return { projects: scanned.map(repoToProject) };
      }
    } catch { /* fall through to API */ }
  }

  const res = await fetch('/api/projects');
  const projects = await res.json();
  return { projects };
};
