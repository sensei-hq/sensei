import { readFile } from "fs/promises";
import { join } from "path";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { IndexResult, ModelBackend } from "@sensei/shared";
import { Scanner } from "./scanner.js";
import { TypeScriptAdapter } from "./adapters/typescript.js";
import { MarkdownAdapter } from "./adapters/markdown.js";
import { Indexer } from "./indexer.js";

export interface IndexRepoOptions {
  repoPath: string;
  repoId: string;
  client: SupabaseClient;
  backend?: ModelBackend;   // optional — enables embedding generation when provided
  include?: string[];
  exclude?: string[];
}

export async function indexRepo(opts: IndexRepoOptions): Promise<IndexResult> {
  const { repoPath, repoId, client, backend } = opts;

  let priorState: Array<{ file_path: string; mtime: number; content_hash: string }> = [];
  try {
    const { data } = await client
      .from("scan_state")
      .select("file_path,mtime,content_hash")
      .eq("repo_id", repoId);
    priorState = (data ?? []) as typeof priorState;
  } catch {
    // First run — no prior state
  }

  const scanner = new Scanner({ repoPath, repoId, priorState, include: opts.include, exclude: opts.exclude });
  const scan = await scanner.scan();

  const tsAdapter = new TypeScriptAdapter();
  const mdAdapter = new MarkdownAdapter();

  // Parse and index TypeScript files
  const parsedTs = await Promise.all(
    scan.files
      .filter(f => scan.changed.includes(f.path) && tsAdapter.extensions.some(ext => f.path.endsWith(ext)))
      .map(f => tsAdapter.parse(f).catch(() => null))
  );
  const validParsedTs = parsedTs.filter((p): p is NonNullable<typeof p> => p !== null);

  const indexer = new Indexer(client);
  const result = await indexer.indexFiles(scan, validParsedTs);

  // Parse and upsert Markdown sections
  const changedMdFiles = scan.files.filter(
    f => scan.changed.includes(f.path) && mdAdapter.extensions.some(ext => f.path.endsWith(ext))
  );

  for (const file of changedMdFiles) {
    try {
      const content = await readFile(join(repoPath, file.path), "utf-8");
      const sections = mdAdapter.parse(file.path, content);
      if (sections.length > 0) {
        await client.from("doc_sections").upsert(
          sections.map(s => ({
            repo_id: repoId,
            file_path: file.path,
            heading: s.heading,
            level: s.level,
            start_line: s.startLine,
            end_line: s.endLine,
            content: s.content,
            code_refs: s.codeRefs,
          })),
          { onConflict: "repo_id,file_path,start_line" }
        );
      }
    } catch {
      // Non-fatal
    }
  }

  // Generate embeddings for changed TS files (best-effort — requires backend)
  if (backend) {
    await Promise.all(
      validParsedTs.map(async parsed => {
        try {
          const chunkText = parsed.symbols.map(s => `${s.name} ${s.signature ?? ""}`.trim()).join(" ");
          if (!chunkText) return;

          const embedding = await backend.embed(chunkText);
          if (embedding.length === 0) return;

          await client.from("embeddings").upsert(
            { repo_id: repoId, file_path: parsed.filePath, chunk_text: chunkText, embedding, updated_at: new Date().toISOString() },
            { onConflict: "repo_id,file_path" }
          );
        } catch {
          // Non-fatal — embedding failure doesn't block indexing
        }
      })
    );
  }

  return result;
}
