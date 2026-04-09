import type { PageLoad } from './$types';

const SENSEI_API = 'http://localhost:7744';

export const load: PageLoad = async ({ fetch }) => {
  let ideas: unknown[] = [];
  try {
    const res = await fetch(`${SENSEI_API}/api/ideas`);
    if (res.ok) ideas = await res.json();
  } catch { /* sensei serve not running */ }

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
