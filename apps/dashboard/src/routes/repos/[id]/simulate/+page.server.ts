// apps/dashboard/src/routes/repos/[id]/simulate/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { getLibDocsTool } from '@sensei/server/tools';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: repoLibs } = await db
    .from('referenced_libraries')
    .select('name,library_id')
    .eq('repo_id', params.id)
    .order('name');

  const libs = ((repoLibs ?? []) as Array<{ name: string; library_id: string | null }>)
    .filter(l => l.library_id)
    .map(l => l.name);

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
  };
};

export const actions: Actions = {
  simulate: async ({ params, request }) => {
    const db = getDb();
    const formData = await request.formData();
    const query = String(formData.get('query') ?? '').trim();
    if (!query) return fail(400, { error: 'Query is required' });

    // Load libs from DB (same as load, but we need them in the action)
    const { data: repoLibs } = await db
      .from('referenced_libraries')
      .select('name,library_id')
      .eq('repo_id', params.id)
      .order('name');

    const libNames = ((repoLibs ?? []) as Array<{ name: string; library_id: string | null }>)
      .filter(l => l.library_id)
      .map(l => l.name);

    if (libNames.length === 0) return fail(400, { error: 'No indexed libraries linked to this repo' });

    try {
      const { TransformersBackend } = await import('@sensei/engine/lib');
      const backend = new TransformersBackend();
      await backend.init();

      // Query all libs in parallel — same as Claude calling get_lib_docs per lib
      const results = await Promise.all(
        libNames.map(async lib => {
          const result = await getLibDocsTool(db as any, backend, params.id, lib, { query, limit: 5 });
          return result.sections.map(s => ({ ...s, lib }));
        })
      );

      // Merge and sort by similarity descending; keyword results (no similarity) go last
      const sections = results
        .flat()
        .sort((a, b) => (b.similarity ?? -1) - (a.similarity ?? -1));

      return { query, sections };
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : String(err) });
    }
  },
};
