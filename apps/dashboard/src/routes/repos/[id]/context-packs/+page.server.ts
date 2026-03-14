import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: packs } = await db
    .from('context_packs')
    .select('id,task,model_id,session_id,total_tokens,slices,created_at')
    .eq('repo_id', params.id)
    .order('created_at', { ascending: false })
    .limit(50);

  return {
    repo: repo as { id: string; name: string },
    packs: (packs ?? []).map(p => ({
      id: p.id as string,
      task: p.task as string,
      modelId: p.model_id as string | null,
      sessionId: p.session_id as string | null,
      totalTokens: p.total_tokens as number,
      slices: (p.slices as any[]) ?? [],
      createdAt: p.created_at as string,
    })),
  };
};
