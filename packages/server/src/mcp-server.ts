import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { makeSenseiClient } from "@sensei/shared";
import { getSessionContext } from "./tools/get-session-context.js";
import { search } from "./tools/search.js";
import { loadContext } from "./tools/load-context.js";

export interface McpServerOptions {
  repoId: string;
  repoPath: string;
}

export function createSenseiMcpServer(opts: McpServerOptions) {
  const server = new McpServer({
    name: "sensei",
    version: "0.1.0",
  });

  // Lazy client — created on first use so startup doesn't block on Supabase connection
  let clientPromise: ReturnType<typeof makeSenseiClient> | null = null;
  const getClient = () => {
    if (!clientPromise) clientPromise = makeSenseiClient(opts.repoPath);
    return clientPromise;
  };

  server.tool(
    "get_session_context",
    "Get orientation context for the current repo — symbol count, stack, last indexed timestamp",
    {},
    async () => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };
        const result = await getSessionContext(client as any, opts.repoId, opts.repoPath);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "search",
    "Search indexed symbols by name, signature, or docstring",
    {
      query: z.string().describe("Search term — matches symbol names, signatures, and docstrings"),
      limit: z.number().int().min(1).max(100).optional().default(20).describe("Max results to return"),
    },
    async ({ query, limit }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await search(client as any, opts.repoId, query, limit);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "load_context",
    "Load the source file at the given path along with its extracted symbols",
    {
      file_path: z.string().describe("Repo-relative file path, e.g. src/index.ts"),
    },
    async ({ file_path }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await loadContext(client as any, opts.repoId, opts.repoPath, file_path);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  return server;
}
