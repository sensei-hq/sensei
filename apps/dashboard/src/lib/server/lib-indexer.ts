// apps/dashboard/src/lib/server/lib-indexer.ts
/**
 * Two-phase library indexer for the dashboard.
 *
 * Phase 1 (startLibFetch): fetch + parse → store sections without embeddings.
 *   Runs on Add / Re-index. No Ollama required. Sections immediately keyword-searchable.
 *
 * Phase 2 (startLibEmbed): generate 384-dim embeddings via TransformersBackend.
 *   Runs on "Build Index". Enables semantic search in getLibDocsTool.
 */
import type { SupabaseClient } from '@supabase/supabase-js';

export interface LibInfo {
  id: string;
  name: string;
  source_type: string;
  base_url: string | null;
  local_path: string | null;
}

// ─── Phase 1: Fetch ──────────────────────────────────────────────────────────

export async function startLibFetch(db: SupabaseClient, lib: LibInfo): Promise<void> {
  await db
    .from('shared_libs')
    .update({ index_status: 'indexing', index_error: null, embed_status: null })
    .eq('id', lib.id);

  runFetch(db, lib).catch(err => {
    console.error(`[lib-indexer] fetch error for ${lib.name}:`, err);
  });
}

async function runFetch(db: SupabaseClient, lib: LibInfo): Promise<void> {
  try {
    const { LlmsTxtAdapter, HttpAdapter, LocalAdapter, GithubAdapter, LibIndexer } =
      await import('@sensei/engine');

    // Use the stored source_type (canonical) — do NOT re-derive from URL
    const sourceType = lib.source_type as 'llms.txt' | 'http' | 'local' | 'github';
    let adapter;
    if (sourceType === 'llms.txt') adapter = new LlmsTxtAdapter();
    else if (sourceType === 'github') adapter = new GithubAdapter();
    else if (sourceType === 'http') adapter = new HttpAdapter();
    else if (sourceType === 'local') adapter = new LocalAdapter();
    else throw new Error(`[lib-indexer] Unknown source_type: "${lib.source_type}"`);

    const entry = {
      name: lib.name,
      source_type: sourceType,
      base_url: lib.base_url ?? undefined,
      local_path: lib.local_path ?? undefined,
    };

    const pages = await adapter.fetch(entry);

    // Phase 1: no backend → sections stored without embeddings
    const { sectionsIndexed } = await new LibIndexer(db, null).indexShared(lib.id, entry, pages);

    await db
      .from('shared_libs')
      .update({
        section_count: sectionsIndexed,
        indexed_at: new Date().toISOString(),
        index_status: 'ready',
        index_error: null,
      })
      .eq('id', lib.id);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[lib-indexer] fetch failed for ${lib.name}:`, msg);
    await db
      .from('shared_libs')
      .update({ index_status: 'error', index_error: msg })
      .eq('id', lib.id);
  }
}

// ─── Phase 2: Embed ──────────────────────────────────────────────────────────

export async function startLibEmbed(db: SupabaseClient, libId: string, libName: string): Promise<void> {
  await db
    .from('shared_libs')
    .update({ embed_status: 'embedding' })
    .eq('id', libId);

  runEmbed(db, libId, libName).catch(err => {
    console.error(`[lib-embedder] embed error for ${libName}:`, err);
  });
}

async function runEmbed(db: SupabaseClient, libId: string, libName: string): Promise<void> {
  try {
    const { TransformersBackend } = await import('@sensei/engine');
    const backend = new TransformersBackend();

    // Fetch sections that need embedding (NULL embedding)
    const { data: sections, error: fetchErr } = await db
      .from('shared_lib_sections')
      .select('id,description,content,source_type')
      .eq('shared_lib_id', libId)
      .is('embedding', null);

    if (fetchErr) throw new Error(fetchErr.message);
    if (!sections || sections.length === 0) {
      await db.from('shared_libs').update({ embed_status: 'ready' }).eq('id', libId);
      return;
    }

    // Embed and update in batches of 20
    const BATCH = 20;
    for (let i = 0; i < sections.length; i += BATCH) {
      const batch = sections.slice(i, i + BATCH);
      await Promise.all(
        batch.map(async (section: { id: string; description: string; content: string | null; source_type: string }) => {
          const input =
            section.source_type === 'llms.txt'
              ? section.description
              : (section.content ?? section.description).slice(0, 512);
          const embedding = await backend.embed(input);
          await db
            .from('shared_lib_sections')
            .update({ embedding })
            .eq('id', section.id);
        })
      );
    }

    await db.from('shared_libs').update({ embed_status: 'ready' }).eq('id', libId);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[lib-embedder] embed failed for ${libName}:`, msg);
    // Don't set index_status to error — sections are still usable via keyword search
    await db.from('shared_libs').update({ embed_status: null }).eq('id', libId);
  }
}

// Keep backward-compatible alias used by existing callers
export const startLibIndexing = startLibFetch;
