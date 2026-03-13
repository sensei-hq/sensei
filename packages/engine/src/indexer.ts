import type { SupabaseClient } from "@supabase/supabase-js";
import type { ScanResult, ParsedFile, IndexResult } from "@sensei/shared";

export class Indexer {
  constructor(private client: SupabaseClient) {}

  async indexFiles(scan: ScanResult, parsed: ParsedFile[]): Promise<IndexResult> {
    const start = Date.now();
    let symbolsUpserted = 0;
    let edgesUpserted = 0;
    let importsUpserted = 0;
    let filesIndexed = 0;
    const errors: string[] = [];

    for (const file of parsed) {
      try {
        // Upsert symbols
        if (file.symbols.length > 0) {
          const symbolRows = file.symbols.map(s => ({
            repo_id: scan.repoId,
            file_path: file.filePath,
            name: s.name,
            kind: s.kind,
            signature: s.signature,
            docstring: s.docstring,
            line_start: s.lineStart,
            line_end: s.lineEnd,
            is_exported: s.isExported,
            updated_at: new Date().toISOString(),
          }));
          const { error } = await this.client
            .from("symbols")
            .upsert(symbolRows, { onConflict: "repo_id,file_path,name,kind" });
          if (error) {
            errors.push(`symbols upsert ${file.filePath}: ${error.message}`);
          } else {
            symbolsUpserted += symbolRows.length;
          }
        }

        // Upsert imports
        if (file.imports.length > 0) {
          const importRows = file.imports.map(i => ({
            repo_id: scan.repoId,
            source_file: file.filePath,
            target_path: i.targetPath,
            names: i.names,
          }));
          const { error } = await this.client
            .from("imports")
            .upsert(importRows, { onConflict: "repo_id,source_file,target_path" });
          if (error) {
            errors.push(`imports upsert ${file.filePath}: ${error.message}`);
          } else {
            importsUpserted += importRows.length;
          }
        }

        // Upsert call edges (requires symbol IDs — look up caller by name first)
        if (file.edges.length > 0) {
          const { data: callerRows } = await this.client
            .from("symbols")
            .select("id,name")
            .eq("repo_id", scan.repoId)
            .eq("file_path", file.filePath);

          const callerIdByName = Object.fromEntries(
            (callerRows ?? []).map(r => [r.name, r.id])
          );

          const edgeRows = file.edges
            .map(e => ({
              repo_id: scan.repoId,
              caller_id: callerIdByName[e.callerName],
              callee_name: e.calleeName,
              callee_file: e.calleeFile,
            }))
            .filter(r => r.caller_id != null);

          if (edgeRows.length > 0) {
            const { error } = await this.client.from("call_edges").insert(edgeRows);
            if (error) {
              errors.push(`call_edges insert ${file.filePath}: ${error.message}`);
            } else {
              edgesUpserted += edgeRows.length;
            }
          }
        }

        filesIndexed++;
      } catch (err) {
        errors.push(`${file.filePath}: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    // Update scan state for all changed files
    if (scan.files.length > 0) {
      const stateRows = scan.files
        .filter(f => scan.changed.includes(f.path))
        .map(f => ({
          repo_id: scan.repoId,
          file_path: f.path,
          mtime: Math.floor(f.mtime),
          content_hash: f.hash,
          indexed_at: new Date().toISOString(),
        }));

      if (stateRows.length > 0) {
        const { error } = await this.client
          .from("scan_state")
          .upsert(stateRows, { onConflict: "repo_id,file_path" });
        if (error) errors.push(`scan_state upsert: ${error.message}`);
      }
    }

    // Remove deleted files from scan_state and symbols
    if (scan.deleted.length > 0) {
      const { error: scanStateDeleteError } = await this.client
        .from("scan_state")
        .delete()
        .eq("repo_id", scan.repoId)
        .in("file_path", scan.deleted);
      if (scanStateDeleteError) {
        errors.push(`scan_state delete: ${scanStateDeleteError.message}`);
      }

      const { error: symbolsDeleteError } = await this.client
        .from("symbols")
        .delete()
        .eq("repo_id", scan.repoId)
        .in("file_path", scan.deleted);
      if (symbolsDeleteError) {
        errors.push(`symbols delete: ${symbolsDeleteError.message}`);
      }
    }

    return {
      repoId: scan.repoId,
      symbolsUpserted,
      edgesUpserted,
      importsUpserted,
      filesIndexed,
      filesDeleted: scan.deleted.length,
      durationMs: Date.now() - start,
      errors,
    };
  }
}
