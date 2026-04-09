import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getSessionContext } from "./tools/get-session-context.js";
import { search } from "./tools/search.js";
import { loadContext } from "./tools/load-context.js";
import { TransformersBackend } from "@sensei/engine";
import { contextPack } from "./tools/context-pack.js";
import { recommendNext } from "./tools/recommend-next.js";
import { tokenStats } from "./tools/token-stats.js";
import { takeSnapshotTool } from "./tools/take-snapshot.js";
import { checkpointTool } from "./tools/checkpoint.js";
import { recordMemoryTool } from "./tools/record-memory.js";
import { closeMemoryTool } from "./tools/close-memory.js";
import { installSkillsTool } from "./tools/install-skills.js";
import { getLibDocsTool } from "./tools/get-lib-docs.js";
import { recordPatternUse } from "./tools/record-pattern-use.js";
import { registerGetSymbolTool } from "./tools/get-symbol-graph.js";
import { getComplexity } from "./tools/get-complexity.js";
import { registerSearchCodeTool } from "./tools/search-code-graph.js";
import { registerGetBearingsTool } from "./tools/get-bearings.js";
import { registerIndexRepoTool } from "./tools/index-repo.js";
import { getActivityLog } from "./activity-log.js";
import { randomUUID } from "node:crypto";

export interface McpServerOptions {
  repoId: string;
  repoPath: string;
}

export function createSenseiMcpServer(opts: McpServerOptions) {
  const server = new McpServer({
    name: "sensei",
    version: "0.1.0",
  });

  let backendInstance: TransformersBackend | null = null;
  const getBackend = () => {
    if (!backendInstance) backendInstance = new TransformersBackend();
    return backendInstance;
  };

  // Session state — stored per MCP server process
  let localSessionId: string | null = null;

  // Lazy — avoids loading native better-sqlite3 until a tool is actually called
  let _activityLog: ReturnType<typeof getActivityLog> | null = null;
  const log = () => {
    if (!_activityLog) _activityLog = getActivityLog(opts.repoId);
    return _activityLog;
  };

  server.tool(
    "get_session_context",
    "Get orientation context for the current repo — symbol count, stack, last indexed timestamp, interrupted sessions, and project memory",
    {
      task_description: z.string().optional().describe("Brief description of the task you are about to work on — used for FTR tracking"),
    },
    async ({ task_description }) => {
      try {
        // Create local session on first call
        if (!localSessionId) {
          localSessionId = log().logSession({
            repoId: opts.repoId,
            task: task_description ?? "session",
            startedAt: new Date().toISOString(),
          });
        }

        const result = await getSessionContext(opts.repoId, opts.repoPath, localSessionId);
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
        const result = await search(opts.repoId, opts.repoPath, query, limit);
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
        const result = await loadContext(opts.repoId, opts.repoPath, file_path);
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
        const result = await contextPack(opts.repoId, opts.repoPath, task, {
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
        const result = await recommendNext(opts.repoId, opts.repoPath, task, model_id);
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
        const result = await tokenStats(opts.repoId, session_id);
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
        const result = await takeSnapshotTool(opts.repoId, randomUUID(), localSessionId ?? undefined, params);
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
        const result = await checkpointTool(
          randomUUID(),
          opts.repoId,
          params,
          opts.repoPath,
          localSessionId ?? undefined,
        );
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
        const result = await recordMemoryTool(
          opts.repoId,
          null,
          params,
          localSessionId ?? undefined,
        );
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
        const result = await closeMemoryTool(opts.repoId, params);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "install_skills",
    "Generate and install project-specific Claude skills derived from the indexed codebase. Requires ANTHROPIC_API_KEY in the environment.",
    {},
    async () => {
      try {
        const result = await installSkillsTool(opts.repoId, opts.repoPath);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "get_lib_docs",
    "Retrieve indexed documentation sections for a library used in this repo. Use for third-party library API lookups.",
    {
      lib:       z.string().describe("Library name as registered in custom_libs"),
      component: z.string().optional().describe("Optional component/section filter"),
      query:     z.string().optional().describe("Semantic search query — omit to list all sections"),
      limit:     z.number().int().min(1).max(50).optional().default(10),
    },
    async ({ lib, component, query, limit }) => {
      try {
        const result = await getLibDocsTool(null, getBackend(), opts.repoId, lib, { component, query, limit });
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  server.tool(
    "record_pattern_use",
    "Record that a named pattern from PATTERNS.md is being applied in this session — call at the start of any pattern-guided implementation",
    {
      pattern_name: z.string().describe("Pattern name exactly as it appears in PATTERNS.md"),
    },
    async ({ pattern_name }) => {
      try {
        const result = await recordPatternUse(opts.repoId, null, pattern_name);
        return { content: [{ type: "text", text: result }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  registerGetSymbolTool(server, opts);
  registerSearchCodeTool(server, opts);
  registerGetBearingsTool(server, opts);
  registerIndexRepoTool(server, opts);

  server.tool(
    "get_complexity",
    "Get complexity hotspots — functions and files ranked by cyclomatic complexity. Use before refactor, optimize, or design tasks.",
    {
      limit: z.number().int().min(1).max(50).optional().default(20).describe("Max hotspots to return"),
      min_complexity: z.number().int().min(1).optional().default(1).describe("Minimum complexity threshold"),
    },
    async ({ limit, min_complexity }) => {
      try {
        const result = await getComplexity(opts.repoId, opts.repoPath, { limit, minComplexity: min_complexity });
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );

  return server;
}
