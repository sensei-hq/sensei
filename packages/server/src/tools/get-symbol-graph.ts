import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getOrCreateDb, getSymbol, type SymbolRef } from "@sensei/graph-indexer";
import type { McpServerOptions } from "../mcp-server.js";

function formatSymbolRefs(refs: SymbolRef[]): string {
  if (!refs || refs.length === 0) return "  (none)";
  return refs.map((r) => `  ${r.name.padEnd(24)} ${r.file}:${r.line}`).join("\n");
}

export function registerGetSymbolTool(server: McpServer, opts: McpServerOptions): void {
  server.tool(
    "get_symbol",
    "Look up a symbol (function or type) in the graph index and return its signature, docstring, callers, and callees",
    {
      name: z.string().describe("Symbol name to look up (e.g. 'parseToken')"),
      depth: z
        .number()
        .int()
        .min(0)
        .max(5)
        .optional()
        .default(1)
        .describe("Depth level 0-5: 0=name only, 1=+sig/docstring, 2=+callers/callees, 3=+imports, 5=+body"),
      project: z
        .string()
        .optional()
        .describe("Filter by project name — use repo name if unsure"),
    },
    async ({ name, depth, project }) => {
      try {
        const { db, conn } = await getOrCreateDb(opts.repoId);
        try {
          const result = await getSymbol(conn, name, project ?? "", depth as 0 | 1 | 2 | 3 | 4 | 5);

          if (!result) {
            return {
              content: [
                {
                  type: "text",
                  text: `Symbol '${name}' not found in project '${project ?? "(any)"}'. Try search_code to find it.`,
                },
              ],
            };
          }

          const lines: string[] = [];
          lines.push(`Symbol: ${result.name} [${result.kind}] ${result.file}:${result.line}`);

          if (result.sig) {
            lines.push(`Signature: ${result.sig}`);
          }
          if (result.docstring) {
            lines.push(`Docstring: ${result.docstring}`);
          }

          if (result.callers !== undefined) {
            lines.push(`\nCallers (${result.callers.length}):`);
            lines.push(formatSymbolRefs(result.callers));
          }

          if (result.callees !== undefined) {
            lines.push(`\nCallees (${result.callees.length}):`);
            lines.push(formatSymbolRefs(result.callees));
          }

          if (result.usedTypes !== undefined && result.usedTypes.length > 0) {
            lines.push(`\nUsed Types (${result.usedTypes.length}):`);
            lines.push(formatSymbolRefs(result.usedTypes));
          }

          if (result.imports !== undefined && result.imports.length > 0) {
            lines.push(`\nImports (${result.imports.length}):`);
            lines.push(result.imports.map((f) => `  ${f}`).join("\n"));
          }

          if (result.importedBy !== undefined && result.importedBy.length > 0) {
            lines.push(`\nImported By (${result.importedBy.length}):`);
            lines.push(result.importedBy.map((f) => `  ${f}`).join("\n"));
          }

          if (result.body) {
            lines.push(`\nBody:\n${result.body}`);
          }

          if (result.comments && result.comments.length > 0) {
            lines.push(`\nComments:`);
            for (const c of result.comments) {
              lines.push(`  [${c.tag}] line ${c.line}: ${c.text}`);
            }
          }

          return { content: [{ type: "text", text: lines.join("\n") }] };
        } finally {
          await conn.close();
          await db.close();
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
