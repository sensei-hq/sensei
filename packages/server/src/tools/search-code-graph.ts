import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getOrCreateDb, searchSymbols } from "@sensei/graph-indexer";
import type { McpServerOptions } from "../mcp-server.js";

export function registerSearchCodeTool(server: McpServer, opts: McpServerOptions): void {
  server.tool(
    "search_code_graph",
    "Search indexed symbols by name using the graph index. Returns functions and types matching the query.",
    {
      query: z.string().describe("Search term — matches symbol names"),
      project: z.string().optional().describe("Filter by project name"),
      limit: z.number().int().min(1).max(50).optional().default(10).describe("Max results to return"),
    },
    async ({ query, project, limit }) => {
      try {
        const { conn } = await getOrCreateDb(opts.repoId);
        try {
          const results = await searchSymbols(conn, query, project ?? "", limit);

          if (results.length === 0) {
            return {
              content: [
                {
                  type: "text",
                  text: `No symbols found matching '${query}'${project ? ` in project '${project}'` : ""}. Try a broader term or run index_repo first.`,
                },
              ],
            };
          }

          const lines: string[] = [`Found ${results.length} symbol(s) matching '${query}':\n`];

          for (const sym of results) {
            lines.push(`[${sym.kind}] ${sym.name}`);
            lines.push(`  File: ${sym.file}:${sym.line}`);
            if (sym.sig) lines.push(`  Sig:  ${sym.sig}`);
            if (sym.docstring) lines.push(`  Doc:  ${sym.docstring}`);
            lines.push("");
          }

          return { content: [{ type: "text", text: lines.join("\n") }] };
        } finally {
          await conn.close();
        }
      } catch (err) {
        return {
          content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }],
          isError: true,
        };
      }
    }
  );
}
