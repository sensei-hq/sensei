// apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { getDb } from '$lib/server/db';
import yaml from 'js-yaml';

type Freshness = 'fresh' | 'stale' | 'missing';

interface LibRow {
  libName: string;
  sourceType: string;
  baseUrl: string | null;
  localPath: string | null;
  sectionCount: number;
  lastFetched: string | null;
  freshness: Freshness;
  skillPath: string | null;
  isShared: boolean;
}

const STALE_DAYS = 7;

function computeFreshness(lastFetched: string | null, sectionCount: number): Freshness {
  if (sectionCount === 0 || !lastFetched) return 'missing';
  const ageMs = Date.now() - new Date(lastFetched).getTime();
  return ageMs / (1000 * 60 * 60 * 24) > STALE_DAYS ? 'stale' : 'fresh';
}

function inferSourceType(input: string): { source_type: 'llms.txt' | 'http' | 'local'; base_url?: string; local_path?: string } {
  if (input.startsWith('http://') || input.startsWith('https://')) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith('/llms.txt') || url.pathname === '/llms.txt') {
        return { source_type: 'llms.txt', base_url: input };
      }
    } catch { /* fall through */ }
    return { source_type: 'http', base_url: input };
  }
  return { source_type: 'local', local_path: input };
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  // All configured libs (includes shared_lib_id for shared libs)
  const { data: repoLibs } = await db
    .from('repo_libs')
    .select('name,source_type,base_url,local_path,skill_path,shared_lib_id')
    .eq('repo_id', params.id);

  // Per-repo indexed sections (only for non-shared libs)
  const { data: sections } = await db
    .from('lib_doc_sections')
    .select('lib_name,last_fetched')
    .eq('repo_id', params.id)
    .limit(10000);

  // Shared lib catalog metadata (section_count, indexed_at)
  const sharedIds = ((repoLibs ?? []) as Array<{ shared_lib_id: string | null }>)
    .map(l => l.shared_lib_id)
    .filter((id): id is string => Boolean(id));

  const { data: sharedCatalog } = sharedIds.length > 0
    ? await db.from('shared_libs').select('id,section_count,indexed_at').in('id', sharedIds)
    : { data: [] as Array<{ id: string; section_count: number; indexed_at: string }> };

  const sharedCatalogMap = new Map(
    ((sharedCatalog ?? []) as Array<{ id: string; section_count: number; indexed_at: string }>)
      .map(s => [s.id, s])
  );

  // Build section map for per-repo libs
  const sectionMap = new Map<string, { count: number; lastFetched: string | null }>();
  for (const s of (sections ?? []) as Array<{ lib_name: string; last_fetched: string }>) {
    const existing = sectionMap.get(s.lib_name);
    if (!existing) {
      sectionMap.set(s.lib_name, { count: 1, lastFetched: s.last_fetched });
    } else {
      existing.count++;
      if (!existing.lastFetched || s.last_fetched > existing.lastFetched) {
        existing.lastFetched = s.last_fetched;
      }
    }
  }

  const libs: LibRow[] = ((repoLibs ?? []) as any[]).map((lib: any) => {
    const sharedLibId: string | null = lib.shared_lib_id ?? null;

    if (sharedLibId) {
      // Shared lib: use catalog metadata; fallback to 0/null if catalog row was deleted
      const catalog = sharedCatalogMap.get(sharedLibId);
      return {
        libName: lib.name,
        sourceType: lib.source_type,
        baseUrl: lib.base_url ?? null,
        localPath: lib.local_path ?? null,
        sectionCount: catalog?.section_count ?? 0,
        lastFetched: catalog?.indexed_at ?? null,
        freshness: 'fresh' as Freshness, // Shared libs don't show freshness
        skillPath: lib.skill_path ?? null,
        isShared: true,
      };
    }

    // Per-repo lib: aggregate from lib_doc_sections
    const info = sectionMap.get(lib.name);
    const sectionCount = info?.count ?? 0;
    const lastFetched = info?.lastFetched ?? null;
    return {
      libName: lib.name,
      sourceType: lib.source_type,
      baseUrl: lib.base_url ?? null,
      localPath: lib.local_path ?? null,
      sectionCount,
      lastFetched,
      freshness: computeFreshness(lastFetched, sectionCount),
      skillPath: lib.skill_path ?? null,
      isShared: false,
    };
  });

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};

export const actions: Actions = {
  add: async ({ params, request }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });
    const repoPath = (repo as { local_path: string }).local_path;

    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    const url = String(formData.get('url') ?? '').trim();

    if (!name) return fail(400, { error: 'Library name is required' });
    if (!url) return fail(400, { error: 'URL or path is required' });

    if (url.startsWith('http://') || url.startsWith('https://')) {
      try { new URL(url); } catch { return fail(400, { error: 'Invalid URL' }); }
    }

    const { source_type, base_url, local_path } = inferSourceType(url);

    // Append to config.yaml so CLI tools can find it
    try {
      const configPath = join(repoPath, '.sensei', 'config.yaml');
      const raw = await readFile(configPath, 'utf-8');
      const config = yaml.load(raw) as Record<string, unknown>;
      const customLibs = (Array.isArray(config.custom_libs) ? config.custom_libs : []) as unknown[];
      if (!customLibs.some((l: unknown) => (l as { name: string }).name === name)) {
        customLibs.push({ name, source_type, ...(base_url ? { base_url } : { local_path }) });
        config.custom_libs = customLibs;
        await writeFile(configPath, yaml.dump(config), 'utf-8');
      }
    } catch (err) {
      return fail(500, { error: `Could not update config.yaml: ${err instanceof Error ? err.message : String(err)}` });
    }

    // Upsert to repo_libs for immediate dashboard visibility
    const { error: upsertErr } = await db.from('repo_libs').upsert(
      { repo_id: params.id, name, source_type, base_url: base_url ?? null, local_path: local_path ?? null },
      { onConflict: 'repo_id,name' }
    );
    if (upsertErr) return fail(500, { error: upsertErr.message });

    // Index the new lib
    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry', '--lib', name],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);
      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || `sensei update-registry --lib ${name} failed` });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },

  reindex: async ({ params, request }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });
    const repoPath = (repo as { local_path: string }).local_path;

    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    if (!name) return fail(400, { error: 'Library name is required' });

    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry', '--lib', name],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);
      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || `sensei update-registry --lib ${name} failed` });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },

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
