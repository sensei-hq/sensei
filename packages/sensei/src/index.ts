import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";
import { reindexRepo } from "./tools/reindex.js";
import { loadContext, recommendNext } from "./tools/context.js";
import { checkDrift } from "./tools/drift.js";
import { checkpoint, getSessionContext, addDecision, addPattern, askQuestion, getOpenItems, closeItem } from "./tools/project-memory.js";
import { SENSEI_DIR } from "./constants.js";

const server = new McpServer({ name: "repo-index-server", version: "0.1.0" });
const REPO = process.env.REPO_PATH ?? process.cwd();

// Query tools
server.tool("get_llmspec", "Get the LLM spec for this repo, optionally a specific section",
  { section: z.string().optional() },
  async ({ section }) => ({ content: [{ type: "text", text: await getLlmSpec(REPO, section) }] })
);

server.tool("get_file_context", "Get a file at a specific resolution level (L0-L3)",
  { path: z.string(), level: z.enum(["L0", "L1", "L2", "L3"]) },
  async ({ path, level }) => ({ content: [{ type: "text", text: await getFileContext(REPO, path, level) }] })
);

server.tool("list_exports", "List all exports at L0, optionally scoped to a module path",
  { module: z.string().optional() },
  async ({ module }) => ({ content: [{ type: "text", text: await listExports(REPO, module) }] })
);

server.tool("find_pattern", "Find a named pattern or list all patterns",
  { name: z.string().optional() },
  async ({ name }) => ({ content: [{ type: "text", text: await findPattern(REPO, name) }] })
);

server.tool("get_shortcuts", "Get dev shortcuts and commands",
  {},
  async () => ({ content: [{ type: "text", text: await getShortcuts(REPO) }] })
);

// Reindex tool
server.tool("reindex_repo", "Scan a repo and build/update the index artifacts",
  { path: z.string().optional() },
  async ({ path }) => {
    const target = path ?? REPO;
    await reindexRepo(target);
    return { content: [{ type: "text", text: `Indexed ${target}. Artifacts written to ${SENSEI_DIR}/` }] };
  }
);

// Context tools (in-session)
server.tool("load_context", "Load a targeted context slice by scope (orientation, patterns, or module path)",
  { scope: z.string() },
  async ({ scope }) => {
    const slice = await loadContext(REPO, scope);
    return { content: [{ type: "text", text: `# Context: ${slice.scope}\n~${slice.tokenEstimate} tokens\n\n${slice.content}` }] };
  }
);

server.tool("recommend_next", "Get recommended minimal context for a given task",
  { task: z.string() },
  async ({ task }) => ({ content: [{ type: "text", text: await recommendNext(REPO, task) }] })
);

// Drift tool
server.tool("check_drift", "Check if indexed docs have drifted from the current state",
  {},
  async () => {
    const result = await checkDrift(REPO);
    return { content: [{ type: "text", text: result.summary }] };
  }
);

// Project memory tools (cross-session)
server.tool("get_session_context", "Load compressed project memory for session resume (~300 tokens)",
  {},
  async () => ({ content: [{ type: "text", text: await getSessionContext(REPO) }] })
);

server.tool("checkpoint", "Distil session and archive. Call at session end or before switching tasks.",
  { summary: z.string(), decisions: z.array(z.string()).optional(), patterns: z.array(z.string()).optional() },
  async ({ summary, decisions, patterns }) => ({
    content: [{ type: "text", text: await checkpoint(REPO, summary, decisions, patterns) }]
  })
);

server.tool("add_decision", "Record a confirmed decision into project memory",
  { text: z.string() },
  async ({ text }) => ({ content: [{ type: "text", text: await addDecision(REPO, text) }] })
);

server.tool("add_pattern", "Record a proven pattern (use when pattern appears 2+ times)",
  { name: z.string(), convention: z.string() },
  async ({ name, convention }) => ({ content: [{ type: "text", text: await addPattern(REPO, name, convention) }] })
);

server.tool("ask_question", "Queue a question for the user (non-blocking)",
  { question: z.string() },
  async ({ question }) => ({
    content: [{ type: "text", text: `Question queued. ID: ${await askQuestion(REPO, question)}` }]
  })
);

server.tool("get_open_items", "Get all unresolved questions and next steps",
  {},
  async () => ({ content: [{ type: "text", text: await getOpenItems(REPO) }] })
);

server.tool("close_item", "Mark an open item as resolved",
  { id: z.string(), resolution: z.string().optional() },
  async ({ id, resolution }) => ({ content: [{ type: "text", text: await closeItem(REPO, id, resolution) }] })
);

// Telemetry tool
server.tool("submit_benchmark_report",
  "Submit an anonymous benchmark report to the sensei telemetry endpoint. Reads SENSEI_TELEMETRY_URL env var (default: http://localhost:7744).",
  { report: z.record(z.unknown()) },
  async ({ report }) => {
    const url = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
    try {
      const res = await fetch(`${url}/reports`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(report),
      });
      const data = await res.json();
      return { content: [{ type: "text", text: `Report submitted: id=${(data as { id?: string }).id ?? "?"}` }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Telemetry unavailable (is sensei serve running?): ${(err as Error).message}` }] };
    }
  }
);

const transport = new StdioServerTransport();
await server.connect(transport);
