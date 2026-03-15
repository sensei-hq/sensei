// apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { LibSkillsManifest, LibSkillFile } from '@sensei/shared';

type Freshness = 'fresh' | 'stale' | 'missing';

interface LibRow {
  libName: string;
  sourceType: string;
  sectionCount: number;
  lastFetched: string | null;
  freshness: Freshness;
  skill: LibSkillFile | null;
}

const STALE_DAYS = 7;

function computeFreshness(lastFetched: string | null): Freshness {
  if (!lastFetched) return 'missing';
  const ageMs = Date.now() - new Date(lastFetched).getTime();
  return ageMs / (1000 * 60 * 60 * 24) > STALE_DAYS ? 'stale' : 'fresh';
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;

  const { data: sections } = await db
    .from('lib_doc_sections')
    .select('lib_name,source_type,last_fetched')
    .eq('repo_id', params.id);

  // Group by lib_name
  const libMap = new Map<string, { sourceType: string; count: number; lastFetched: string | null }>();
  for (const row of (sections ?? []) as Array<{ lib_name: string; source_type: string; last_fetched: string }>) {
    const existing = libMap.get(row.lib_name);
    if (!existing) {
      libMap.set(row.lib_name, { sourceType: row.source_type, count: 1, lastFetched: row.last_fetched });
    } else {
      existing.count++;
      if (!existing.lastFetched || row.last_fetched > existing.lastFetched) {
        existing.lastFetched = row.last_fetched;
      }
    }
  }

  // Load lib-skills manifest
  const manifestPath = join(repoPath, '.sensei', 'lib-skills.json');
  const skillsByLib = new Map<string, LibSkillFile>();
  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as LibSkillsManifest;
      for (const s of manifest.skills) skillsByLib.set(s.libName, s);
    } catch { /* ignore malformed */ }
  }

  const libs: LibRow[] = Array.from(libMap.entries()).map(([libName, info]) => ({
    libName,
    sourceType: info.sourceType,
    sectionCount: info.count,
    lastFetched: info.lastFetched,
    freshness: computeFreshness(info.lastFetched),
    skill: skillsByLib.get(libName) ?? null,
  }));

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};

export const actions: Actions = {
  update: async ({ params }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });

    const repoPath = (repo as { local_path: string }).local_path;
    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry'],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || 'sensei update-registry failed' });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },
};
