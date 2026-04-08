import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const ideas = await fetch('/api/ideas').then(r => r.json());

  // Also surface repos tagged as 'idea' from the scan
  let ideaRepos: Array<{ name: string; path: string; description: string | null; status: string; tech_stack: string[]; last_commit_days: number | null }> = [];
  const raw = typeof localStorage !== 'undefined' ? localStorage.getItem('sensei:projects_raw') : null;
  if (raw) {
    try {
      const all = JSON.parse(raw);
      ideaRepos = all.filter((r: { categories?: string[] }) => r.categories?.includes('idea'));
    } catch { /* ignore */ }
  }

  return { ideas, ideaRepos };
};
