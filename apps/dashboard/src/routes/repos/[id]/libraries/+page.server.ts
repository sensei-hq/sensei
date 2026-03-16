// apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { getDb } from '$lib/server/db';
import { startLibIndexing } from '$lib/server/lib-indexer';
import { inferSourceType } from '@sensei/engine';
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
  sharedLibId: string | null;
}

const STALE_DAYS = 7;

function computeFreshness(lastFetched: string | null, sectionCount: number): Freshness {
  if (sectionCount === 0 || !lastFetched) return 'missing';
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
    ? await db.from('shared_libs').select('id,section_count,indexed_at,index_status').in('id', sharedIds)
    : { data: [] as Array<{ id: string; section_count: number; indexed_at: string; index_status: string }> };

  const sharedCatalogMap = new Map(
    ((sharedCatalog ?? []) as Array<{ id: string; section_count: number; indexed_at: string; index_status: string }>)
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
      const catalog = sharedCatalogMap.get(sharedLibId);
      return {
        libName: lib.name,
        sourceType: lib.source_type,
        baseUrl: lib.base_url ?? null,
        localPath: lib.local_path ?? null,
        sectionCount: catalog?.section_count ?? 0,
        lastFetched: catalog?.indexed_at ?? null,
        freshness: computeFreshness(catalog?.indexed_at ?? null, catalog?.section_count ?? 0),
        skillPath: lib.skill_path ?? null,
        isShared: true,
        sharedLibId: sharedLibId,
      };
    }

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
      sharedLibId: null,
    };
  });

  // Fetch catalog for linking (mark already-linked ones)
  const linkedIds = new Set(libs.filter((l: any) => l.sharedLibId).map((l: any) => l.sharedLibId!));
  const { data: catalogData } = await db
    .from('shared_libs')
    .select('id,name,source_type,icon_url,category,section_count,index_status')
    .order('name');

  const catalog = (catalogData ?? []).map((c: any) => ({ ...c, linked: linkedIds.has(c.id) }));

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    catalog,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};

export const actions: Actions = {
  /**
   * Register a lib in the shared pool and link this repo to it.
   * No indexing happens here — click Re-index to fetch docs.
   */
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

    const inferred = inferSourceType(url);
    const { source_type } = inferred;
    const base_url = 'base_url' in inferred ? inferred.base_url : undefined;
    const local_path = 'local_path' in inferred ? inferred.local_path : undefined;

    // Update config.yaml
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

    // Upsert to shared_libs (by name — idempotent)
    const { data: sharedLib, error: sharedLibErr } = await db
      .from('shared_libs')
      .upsert(
        { name, source_type, base_url: base_url ?? null, local_path: local_path ?? null },
        { onConflict: 'name' }
      )
      .select('id')
      .single();

    if (sharedLibErr || !sharedLib) {
      return fail(500, { error: `shared_libs upsert failed: ${sharedLibErr?.message}` });
    }

    // Link this repo to the shared lib
    const { error: upsertErr } = await db.from('repo_libs').upsert(
      {
        repo_id: params.id,
        name,
        source_type,
        base_url: base_url ?? null,
        local_path: local_path ?? null,
        shared_lib_id: sharedLib.id,
      },
      { onConflict: 'repo_id,name' }
    );
    if (upsertErr) return fail(500, { error: upsertErr.message });

    // Background index of the shared lib
    await startLibIndexing(db, {
      id: sharedLib.id,
      name,
      source_type,
      base_url: base_url ?? null,
      local_path: local_path ?? null,
    });

    redirect(303, `/repos/${params.id}/libraries`);
  },

  /** Link an existing shared lib from the catalog to this repo. */
  link: async ({ params, request }) => {
    const db = getDb();
    const formData = await request.formData();
    const sharedLibId = String(formData.get('shared_lib_id') ?? '').trim();
    if (!sharedLibId) return fail(400, { error: 'shared_lib_id is required' });

    const { data: lib } = await db
      .from('shared_libs')
      .select('id,name,source_type,base_url,local_path')
      .eq('id', sharedLibId)
      .single();
    if (!lib) return fail(404, { error: 'Library not found in catalog' });

    const { error: linkErr } = await db.from('repo_libs').upsert({
      repo_id: params.id,
      shared_lib_id: lib.id,
      name: lib.name,
      source_type: lib.source_type,
      base_url: lib.base_url ?? null,
      local_path: lib.local_path ?? null,
    }, { onConflict: 'repo_id,name' });
    if (linkErr) return fail(500, { error: linkErr.message });

    return { linked: true };
  },

  /** Trigger background re-index for a single lib. */
  reindex: async ({ params, request }) => {
    const db = getDb();
    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    if (!name) return fail(400, { error: 'Library name is required' });

    const { data: libRow } = await db
      .from('repo_libs')
      .select('source_type,base_url,local_path,shared_lib_id')
      .eq('repo_id', params.id)
      .eq('name', name)
      .single();

    if (!libRow) return fail(404, { error: `Library '${name}' not found` });

    const sharedLibId: string | null = (libRow as any).shared_lib_id ?? null;
    if (sharedLibId) {
      await startLibIndexing(db, {
        id: sharedLibId,
        name,
        source_type: (libRow as any).source_type,
        base_url: (libRow as any).base_url ?? null,
        local_path: (libRow as any).local_path ?? null,
      });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },

  /** Trigger background re-index for all libs in this repo. */
  update: async ({ params }) => {
    const db = getDb();
    const { data: repoLibs } = await db
      .from('repo_libs')
      .select('name,source_type,base_url,local_path,shared_lib_id')
      .eq('repo_id', params.id);

    if (!repoLibs?.length) redirect(303, `/repos/${params.id}/libraries`);

    for (const lib of (repoLibs ?? []) as any[]) {
      if (lib.shared_lib_id) {
        await startLibIndexing(db, {
          id: lib.shared_lib_id,
          name: lib.name,
          source_type: lib.source_type,
          base_url: lib.base_url ?? null,
          local_path: lib.local_path ?? null,
        });
      }
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },
};
