// apps/dashboard/src/routes/repos/[id]/drift/+page.server.ts
import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';
import { checkDrift } from '@sensei/tools';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path,last_indexed_at')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  if (!repo.local_path) {
    return {
      repo: repo as { id: string; name: string; local_path: string | null; last_indexed_at: string | null },
      drifted: [],
      summary: 'No local path configured for this repo.',
      lastIndexedCommit: null,
    };
  }

  try {
    const result = await checkDrift(repo.local_path);
    return {
      repo: repo as { id: string; name: string; local_path: string | null; last_indexed_at: string | null },
      drifted: result.drifted,
      summary: result.summary,
      lastIndexedCommit: result.lastIndexedCommit ?? null,
    };
  } catch (err) {
    return {
      repo: repo as { id: string; name: string; local_path: string | null; last_indexed_at: string | null },
      drifted: [],
      summary: `Error running drift check: ${err instanceof Error ? err.message : String(err)}`,
      lastIndexedCommit: null,
    };
  }
};
