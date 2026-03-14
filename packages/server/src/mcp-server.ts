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
import { takeSnapshotTool } from "./tools/take-snapshot.js";
import { checkpointTool } from "./tools/checkpoint.js";
import { recordMemoryTool } from "./tools/record-memory.js";
import { closeMemoryTool } from "./tools/close-memory.js";
import { createSession, updateHeartbeat, createTaskSession, recordTaskTurn, completeTaskSession } from "@sensei/engine";

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

  // Session state — stored per MCP server process
  let sessionId: string | null = null;
  let taskSessionId: string | null = null;

  const beat = (client: any, toolName: string, success: boolean) => {
    if (sessionId) updateHeartbeat(client, sessionId).catch(() => {});
    if (taskSessionId) recordTaskTurn(client as any, taskSessionId, opts.repoId, toolName, success).catch(() => {});
  };

  server.tool(
    "get_session_context",
    "Get orientation context for the current repo — symbol count, stack, last indexed timestamp, interrupted sessions, and project memory",
    {
      task_description: z.string().optional().describe("Brief description of the task you are about to work on — used for FTR tracking"),
    },
    async ({ task_description }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };

        // Create session on first call; reuse on subsequent calls (idempotent)
        // Assign sessionId and taskSessionId atomically — both or neither
        if (!sessionId) {
          const session = await createSession(client as any, opts.repoId);
          const taskSession = await createTaskSession(client as any, session.id, opts.repoId, task_description);
          sessionId = session.id;
          taskSessionId = taskSession.id;
        }

        const result = await getSessionContext(client as any, opts.repoId, opts.repoPath, sessionId);
        beat(client, "get_session_context", true);
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
        beat(client, "search", true);
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
        beat(client, "load_context", true);
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
        beat(client, "context_pack", true);
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
        beat(client, "recommend_next", true);
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
        beat(client, "token_stats", true);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "take_snapshot",
    "Save a snapshot of current progress — use at step boundaries so the next session can recover if interrupted",
    {
      progress_summary: z.string().describe("What you are doing right now"),
      next_step_hint: z.string().optional().describe("What to do next if interrupted"),
      in_flight_files: z.array(z.string()).optional().describe("Files currently being modified"),
      completed_steps: z.array(z.string()).optional().describe("Steps finished so far this task"),
      worktree_refs: z.array(z.object({ branch: z.string(), path: z.string(), status: z.string() })).optional().describe("Active git worktrees"),
      diff_stat_summary: z.string().optional().describe("e.g. '8 files changed, +142 -31'"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await takeSnapshotTool(client as any, sessionId, opts.repoId, params);
        beat(client, "take_snapshot", true);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "checkpoint",
    "Mark a task complete — writes a final snapshot and closes the session. Call when a coherent unit of work is done.",
    {
      task_summary: z.string().describe("What was accomplished"),
      completed_steps: z.array(z.string()).optional().describe("Final list of completed steps"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await checkpointTool(client as any, sessionId, opts.repoId, params);
        // completeTaskSession runs after checkpointTool so sessions.status is already 'completed'
        // Throws on DB error — the outer try/catch surfaces it to the agent as isError: true
        if (taskSessionId) {
          await completeTaskSession(client as any, taskSessionId, sessionId);
        }
        beat(client, "checkpoint", true);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "record_memory",
    "Store a decision, pattern, or open question in project memory — survives across sessions",
    {
      type: z.enum(["decision", "pattern", "question"]).describe("decision=architectural choice, pattern=coding convention, question=open question to resolve"),
      title: z.string().describe("Short label, e.g. 'Use optimistic locking for invoice updates'"),
      content: z.string().describe("Full description"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        if (!sessionId) return { content: [{ type: "text", text: "Error: No active session. Call get_session_context first." }], isError: true };
        const result = await recordMemoryTool(client as any, opts.repoId, sessionId, params);
        beat(client, "record_memory", true);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "close_memory",
    "Resolve an open question in project memory",
    {
      id: z.string().describe("Memory item ID returned by record_memory"),
      resolution: z.string().describe("How the question was resolved"),
    },
    async (params) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await closeMemoryTool(client as any, params);
        beat(client, "close_memory", true);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  return server;
}
