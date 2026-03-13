import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';

export const load: PageServerLoad = async () => {
  const db = getDb();

  const { data: repos } = await db
    .from('repos')
    .select('id,name,local_path,stack,last_indexed_at,created_at')
    .order('created_at', { ascending: false });

  const symbolCounts = await Promise.all(
    (repos ?? []).map(async repo => {
      const [{ count: symbolCount }, { count: fileCount }] = await Promise.all([
        db.from('symbols').select('*', { count: 'exact', head: true }).eq('repo_id', repo.id),
        db.from('scan_state').select('*', { count: 'exact', head: true }).eq('repo_id', repo.id),
      ]);
      return { repo_id: repo.id, symbolCount: symbolCount ?? 0, fileCount: fileCount ?? 0 };
    })
  );

  const countMap = Object.fromEntries(symbolCounts.map(c => [c.repo_id, c]));

  return {
    repos: (repos ?? []).map(r => ({
      ...r,
      symbol_count: countMap[r.id]?.symbolCount ?? 0,
      file_count: countMap[r.id]?.fileCount ?? 0,
    })),
  };
};
