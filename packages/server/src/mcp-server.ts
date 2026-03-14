import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { makeSenseiClient } from "@sensei/shared";
import { getSessionContext } from "./tools/get-session-context.js";
import { search } from "./tools/search.js";
import { loadContext } from "./tools/load-context.js";
import { OllamaBackend } from "./model/ollama-backend.js";
import { contextPack } from "./tools/context-pack.js";
import { recommendNext } from "./tools/recommend-next.js";
import { tokenStats } from "./tools/token-stats.js";

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

  let backendInstance: OllamaBackend | null = null;
  const getBackend = () => {
    if (!backendInstance) backendInstance = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });
    return backendInstance;
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

  server.tool(
    "context_pack",
    "Get a ranked, token-budgeted set of code and doc slices relevant to a task. More precise than load_context — use this when you need focused context for a specific task.",
    {
      task: z.string().describe("Task description — the code problem or question you are working on"),
      max_tokens: z.number().int().min(100).max(32000).optional().default(8000).describe("Maximum tokens to include"),
      model_id: z.string().optional().describe("Your model ID (e.g. 'claude-sonnet-4-6') — selects the right token counter"),
      session_id: z.string().optional().describe("Session ID for grouping packs in the dashboard"),
      session_context: z.array(z.string()).optional().describe("File paths already in your context — excluded from the pack"),
    },
    async ({ task, max_tokens, model_id, session_id, session_context }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };
        const result = await contextPack(client as any, getBackend(), opts.repoId, opts.repoPath, task, {
          maxTokens: max_tokens,
          modelId: model_id,
          sessionId: session_id,
          sessionContext: session_context,
        });
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "recommend_next",
    "Get the top 3 most relevant files for a task with estimated token counts and a suggested budget for context_pack",
    {
      task: z.string().describe("Task description"),
      model_id: z.string().optional().describe("Your model ID for accurate token counting"),
    },
    async ({ task, model_id }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await recommendNext(client as any, getBackend(), opts.repoId, task, model_id);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "token_stats",
    "Get token usage statistics for a session — total packs requested and tokens served",
    {
      session_id: z.string().describe("Session ID to look up stats for"),
    },
    async ({ session_id }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await tokenStats(client as any, session_id);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  return server;
}
