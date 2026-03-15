// apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.server.ts
import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { LibSkillsManifest } from '@sensei/shared';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos').select('id,name,local_path').eq('id', params.id).single();

  if (!repo) throw error(404, 'Repo not found');
  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;

  const { data: sections, error: dbError } = await db
    .from('lib_doc_sections')
    .select('title,url,local_path,description,content,source_type,component,last_fetched')
    .eq('repo_id', params.id)
    .eq('lib_name', params.name)
    .order('title');

  if (dbError) throw error(500, dbError.message);

  const manifestPath = join(repoPath, '.sensei', 'lib-skills.json');
  let skillPath: string | null = null;
  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as LibSkillsManifest;
      const found = manifest.skills.find(s => s.libName === params.name);
      if (found && existsSync(found.path)) skillPath = found.path;
    } catch { /* ignore */ }
  }

  return {
    repo: repo as { id: string; name: string },
    libName: params.name,
    sections: sections ?? [],
    skillPath,
  };
};
