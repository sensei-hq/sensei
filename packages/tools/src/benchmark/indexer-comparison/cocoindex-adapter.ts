import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import type { CocoChunk } from "./types.js";

export interface CocoIndex {
  files: string[];
  search: (query: string, limit?: number) => Promise<CocoChunk[]>;
  close: () => Promise<void>;
}

export async function connectCocoindex(repoPath: string): Promise<CocoIndex> {
  // Set cwd so cocoindex-code reads .cocoindex_code/ from the right repo root.
  // Do NOT use COCOINDEX_DIR — that env var is not documented/supported.
  const transport = new StdioClientTransport({
    command: "cocoindex-code",
    args: ["serve"],
    env: Object.fromEntries(Object.entries(process.env).filter(([, v]) => v !== undefined)) as Record<string, string>,
    cwd: repoPath,
  });

  const client = new Client({ name: "sensei-benchmark", version: "1.0.0" });
  await client.connect(transport);

  const normalize = (filePath: string): string => {
    const prefix = repoPath.endsWith("/") ? repoPath : repoPath + "/";
    return filePath.startsWith(prefix) ? filePath.slice(prefix.length) : filePath;
  };

  // Wait for index to be ready (retry up to 30s)
  const deadline = Date.now() + 30_000;
  let ready = false;
  while (Date.now() < deadline) {
    const result = await client.callTool({
      name: "search",
      arguments: { query: "test", limit: 1, refresh_index: false },
    }) as { content: Array<{ text: string }> };
    try {
      const parsed = JSON.parse(result.content[0]?.text ?? "{}");
      if (parsed.success) { ready = true; break; }
    } catch {
      // malformed response — keep retrying
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  if (!ready) {
    await client.close();
    throw new Error("cocoindex-code: index not ready after 30s. Run: cocoindex-code index");
  }

  // Collect all indexed file paths via a broad search
  const allFiles = new Set<string>();
  const probeResult = await client.callTool({
    name: "search",
    arguments: { query: "function", limit: 100, refresh_index: false },
  }) as { content: Array<{ text: string }> };
  try {
    const probeData = JSON.parse(probeResult.content[0]?.text ?? "{}");
    if (probeData.success) {
      for (const r of probeData.results) allFiles.add(r.file_path);
    }
  } catch {
    // ignore malformed probe response
  }

  // Normalize paths: strip repoPath prefix so paths are relative (matching ground truth format)
  const normalizedFiles = Array.from(allFiles).map(f => normalize(f));

  return {
    files: normalizedFiles,
    async search(query: string, limit = 5): Promise<CocoChunk[]> {
      const res = await client.callTool({
        name: "search",
        arguments: { query, limit, refresh_index: false },
      }) as { content: Array<{ text: string }> };
      try {
        const data = JSON.parse(res.content[0]?.text ?? "{}");
        if (!data.success) return [];
        return data.results.map((r: {
          file_path: string; language: string; content: string;
          start_line: number; end_line: number; score: number;
        }) => ({
          filePath: normalize(r.file_path), language: r.language, content: r.content,
          startLine: r.start_line, endLine: r.end_line, score: r.score,
        }));
      } catch {
        return [];
      }
    },
    async close() {
      await client.close();
    },
  };
}
