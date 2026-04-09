import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { indexRepo } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";
import type { McpServerOptions } from "../mcp-server.js";

export function registerIndexRepoTool(server: McpServer, opts: McpServerOptions): void {
  server.tool(
    "index_repo",
    "Trigger a fresh graph index of the current repo. Use after significant code changes to keep the graph up to date.",
    {
      project: z
        .string()
        .optional()
        .describe("Project display name — defaults to repo name from config"),
    },
    async ({ project }) => {
      try {
        const config = await loadSenseiConfig(opts.repoPath);
        const projectName = project ?? config?.repo_id ?? opts.repoId;

        const startMsg = `Indexing repo '${projectName}' at ${opts.repoPath} ...`;
        
        const result = await indexRepo({
          repoPath: opts.repoPath,
          repoId: opts.repoId,
          project: projectName,
        });

        const lines: string[] = [
          startMsg,
          "",
          "Index complete:",
          `  Files indexed:     ${result.filesIndexed}`,
          `  Functions indexed: ${result.functionsIndexed}`,
          `  Types indexed:     ${result.typesIndexed}`,
          `  Edges created:     ${result.edgesCreated}`,
          `  Duration:          ${result.durationMs}ms`,
          "",
          `Graph is ready. Use get_symbol(name) or search_code_graph(query) to explore.`,
        ];

        return { content: [{ type: "text", text: lines.join("\n") }] };
      } catch (err) {
        return {
          content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
          isError: true,
        };
      }
    }
  );
}
