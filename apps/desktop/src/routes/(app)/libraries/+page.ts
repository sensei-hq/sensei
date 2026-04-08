import type { PageLoad } from './$types';

type ScannedRepo = {
  name: string; path: string; remote: string | null; description: string | null;
  categories: string[]; status: string; last_commit_days: number | null;
  tech_stack: string[]; commit_count: number;
  duplicate_of: string | null; variant_group: string | null;
  client?: string | null;
};

export const load: PageLoad = async () => {
  const raw = typeof localStorage !== 'undefined' ? localStorage.getItem('sensei:projects_raw') : null;
  if (raw) {
    try {
      const all: ScannedRepo[] = JSON.parse(raw);
      const libs = all.filter(r => r.categories?.includes('library'));
      return { libraries: libs };
    } catch { /* fall through */ }
  }
  return { libraries: [] };
};
