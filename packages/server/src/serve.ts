import { mkdir, readFile, writeFile } from "fs/promises";
import { dirname, join, relative } from "path";
import { homedir } from "os";
import { intro, log } from "@clack/prompts";
import { loadSenseiConfig } from "@sensei/shared";
import { watchRepo, getOrCreateDb, indexRepo } from "@sensei/graph-indexer";
import { checkSystemRequirements, OLLAMA_BASE_URL, OLLAMA_MODEL } from "./model/system-check.js";
import { OllamaBackend, makeFallbackAnalysis } from "./model/ollama-backend.js";
import { getActivityLog } from "./activity-log.js";

const PROJECTS_FILE = join(homedir(), ".sensei", "projects.json");

export interface ProjectEntry {
  repoId: string;
  name: string;
  path: string;
  indexedAt?: string;
}

async function readProjects(): Promise<ProjectEntry[]> {
  try {
    return JSON.parse(await readFile(PROJECTS_FILE, "utf-8"));
  } catch {
    return [];
  }
}

async function writeProjects(projects: ProjectEntry[]): Promise<void> {
  await mkdir(dirname(PROJECTS_FILE), { recursive: true });
  await writeFile(PROJECTS_FILE, JSON.stringify(projects, null, 2));
}

function corsHeaders() {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
  };
}

function jsonResponse(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json", ...corsHeaders() },
  });
}

/** Query Kuzu graph for graph intelligence data (communities, godNodes, rationale). */
async function buildGraphData(repoId: string, repoPath: string) {
  const { db, conn } = await getOrCreateDb(repoId);
  try {
    // Total symbols
    const symResult = await conn.query(
      `MATCH (f:Function {project: '${repoId}'}) RETURN COUNT(f) AS cnt`
    );
    const symRows = Array.isArray(symResult) ? symResult[0] : symResult;
    const symData = await symRows.getAll() as Record<string, unknown>[];
    const totalSymbols = Number(symData[0]?.["cnt"] ?? 0);

    // All functions
    const fnResult = await conn.query(
      `MATCH (f:Function {project: '${repoId}'}) RETURN f.id AS id, f.name AS name, f.file AS file, f.line AS line`
    );
    const fnRows = Array.isArray(fnResult) ? fnResult[0] : fnResult;
    const functions = (await fnRows.getAll() as Record<string, unknown>[]).map((r) => ({
      id: String(r["id"]),
      name: String(r["name"]),
      file: String(r["file"]),
      line: Number(r["line"]),
    }));

    // CALLS edges
    let totalEdges = 0;
    const degreeMap = new Map<string, number>();
    try {
      const callResult = await conn.query(
        `MATCH (a:Function {project: '${repoId}'})-[:CALLS]->(b:Function {project: '${repoId}'}) RETURN a.id AS src, b.id AS dst`
      );
      const callRows = Array.isArray(callResult) ? callResult[0] : callResult;
      const edges = await callRows.getAll() as Record<string, unknown>[];
      totalEdges = edges.length;
      for (const e of edges) {
        const src = String(e["src"]);
        const dst = String(e["dst"]);
        degreeMap.set(src, (degreeMap.get(src) ?? 0) + 1);
        degreeMap.set(dst, (degreeMap.get(dst) ?? 0) + 1);
      }
    } catch { /* no edges yet */ }

    // Comment / rationale nodes
    const rationaleItems: Array<{ file: string; tag: string; project: string; text: string }> = [];
    try {
      const comResult = await conn.query(
        `MATCH (c:Comment {project: '${repoId}'}) RETURN c.text AS text, c.tag AS tag, c.file AS file`
      );
      const comRows = Array.isArray(comResult) ? comResult[0] : comResult;
      const comData = await comRows.getAll() as Record<string, unknown>[];
      for (const r of comData) {
        const tag = String(r["tag"] ?? "NOTE").toUpperCase();
        if (["WHY", "DECISION", "HACK", "NOTE"].includes(tag)) {
          rationaleItems.push({
            file: relative(repoPath, String(r["file"])),
            tag,
            project: repoId,
            text: String(r["text"]),
          });
        }
      }
    } catch { /* no comment nodes yet */ }

    // Build communities by grouping functions into top-level directory clusters
    const COLORS = ["bg-primary-z5", "bg-secondary-z5", "bg-success-z5", "bg-warning-z5", "bg-info-z5", "bg-danger-z5"];
    const dirMap = new Map<string, typeof functions>();
    for (const fn of functions) {
      const rel = relative(repoPath, fn.file);
      const parts = rel.split("/");
      const dir = parts.length > 1 ? parts[0] + (parts.length > 2 ? `/${parts[1]}` : "") : "(root)";
      if (!dirMap.has(dir)) dirMap.set(dir, []);
      dirMap.get(dir)!.push(fn);
    }

    // God nodes = top 20 by degree across all directories
    const godNodeIds = new Set(
      [...degreeMap.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 20)
        .map(([id]) => id)
    );
    const fnById = new Map(functions.map((f) => [f.id, f]));

    const godNodes = [...godNodeIds].map((id) => {
      const fn = fnById.get(id)!;
      const rel = relative(repoPath, fn.file);
      const parts = rel.split("/");
      const community = parts.length > 1 ? parts[0] + (parts.length > 2 ? `/${parts[1]}` : "") : "(root)";
      return {
        name: fn.name,
        project: repoId,
        degree: degreeMap.get(id) ?? 0,
        community,
        file: rel,
      };
    }).sort((a, b) => b.degree - a.degree);

    let colorIdx = 0;
    const communities = [...dirMap.entries()].map(([dir, fns]) => {
      const communityGodNodes = fns
        .filter((f) => godNodeIds.has(f.id))
        .sort((a, b) => (degreeMap.get(b.id) ?? 0) - (degreeMap.get(a.id) ?? 0))
        .slice(0, 5)
        .map((f) => f.name);
      return {
        id: dir,
        label: dir,
        project: repoId,
        color: COLORS[colorIdx++ % COLORS.length],
        symbolCount: fns.length,
        godNodes: communityGodNodes,
      };
    }).sort((a, b) => b.symbolCount - a.symbolCount);

    return {
      summary: { totalSymbols, totalEdges, communities: communities.length },
      projects: [repoId],
      communities,
      godNodes,
      rationale: rationaleItems,
    };
  } finally {
    await conn.close();
    await db.close();
  }
}

export interface ServeOptions {
  port?: number;
  repoPath?: string;
  repoId?: string;
  isAvailableFn?: () => Promise<{ ollamaRunning: boolean; ollamaModel: boolean }>;
  ollamaBackendFn?: () => OllamaBackend;
}

export async function createReportServer(opts: ServeOptions = {}): Promise<{ stop: () => void; port: number }> {
  const port = opts.port ?? 7744;
  const activeRepoId = opts.repoId ?? null;
  const activeRepoPath = opts.repoPath ?? null;

  const server = Bun.serve({
    port,
    async fetch(req: Request) {
      const url = new URL(req.url);

      // CORS preflight
      if (req.method === "OPTIONS") {
        return new Response(null, { status: 204, headers: corsHeaders() });
      }

      // ── REST API ───────────────────────────────────────────────────────────

      if (url.pathname === "/api/projects") {
        if (req.method === "GET") {
          const projects = await readProjects();
          return jsonResponse(projects);
        }
        if (req.method === "POST") {
          try {
            const body = await req.json() as Partial<ProjectEntry>;
            if (!body.repoId || !body.path) {
              return jsonResponse({ ok: false, error: "repoId and path are required" }, 400);
            }
            const projects = await readProjects();
            const idx = projects.findIndex((p) => p.repoId === body.repoId);
            const entry: ProjectEntry = {
              repoId: body.repoId,
              name: body.name ?? body.repoId,
              path: body.path,
              indexedAt: body.indexedAt,
            };
            if (idx >= 0) projects[idx] = entry; else projects.push(entry);
            await writeProjects(projects);
            return jsonResponse({ ok: true });
          } catch {
            return jsonResponse({ ok: false, error: "Invalid JSON" }, 400);
          }
        }
      }

      if (req.method === "GET" && url.pathname === "/api/graph") {
        const repoId = url.searchParams.get("repoId") ?? activeRepoId;
        const repoPath = url.searchParams.get("repoPath") ?? activeRepoPath;
        if (!repoId || !repoPath) {
          return jsonResponse({ summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, projects: [], communities: [], godNodes: [], rationale: [] });
        }
        try {
          const data = await buildGraphData(repoId, repoPath);
          return jsonResponse(data);
        } catch (err) {
          return jsonResponse({ ok: false, error: (err as Error).message }, 500);
        }
      }

      if (req.method === "GET" && url.pathname === "/api/sessions") {
        const repoId = url.searchParams.get("repoId") ?? activeRepoId;
        if (!repoId) {
          return jsonResponse({ stats: null, sessions: [], toolUsage: [], benchmarkPairs: [] });
        }
        try {
          const log = getActivityLog(repoId);
          const recent = log.getRecentSessions(50);
          const stats = log.getStats();
          const sessions = recent.map((s) => ({
            ...s,
            project: s.repoId,
            ftr: s.outcome === "completed" ? 1.0 : s.outcome === "partial" ? 0.5 : s.outcome === "blocked" ? 0.0 : null,
          }));
          return jsonResponse({ stats, sessions, toolUsage: [], benchmarkPairs: [] });
        } catch (err) {
          return jsonResponse({ ok: false, error: (err as Error).message }, 500);
        }
      }

      if (req.method === "GET" && url.pathname === "/api/ideas") {
        const repoId = url.searchParams.get("repoId") ?? activeRepoId;
        if (!repoId) return jsonResponse([]);
        try {
          const log = getActivityLog(repoId);
          const items = log.getOpenBacklog();
          return jsonResponse(items);
        } catch (err) {
          return jsonResponse({ ok: false, error: (err as Error).message }, 500);
        }
      }

      if (req.method === "POST" && url.pathname === "/api/index") {
        let body: { repoId?: string; repoPath?: string };
        try {
          body = await req.json() as typeof body;
        } catch {
          return jsonResponse({ ok: false, error: "Invalid JSON" }, 400);
        }
        const repoId = body.repoId ?? activeRepoId;
        const repoPath = body.repoPath ?? activeRepoPath;
        if (!repoId || !repoPath) {
          return jsonResponse({ ok: false, error: "repoId and repoPath are required" }, 400);
        }
        const stream = new ReadableStream({
          async start(controller) {
            const send = (obj: unknown) =>
              controller.enqueue(new TextEncoder().encode(JSON.stringify(obj) + "\n"));
            try {
              send({ status: "indexing", repoId });
              await indexRepo({ repoId, repoPath, project: repoId });
              // Register project
              const projects = await readProjects();
              const idx = projects.findIndex((p) => p.repoId === repoId);
              const entry: ProjectEntry = { repoId, name: repoId, path: repoPath, indexedAt: new Date().toISOString() };
              if (idx >= 0) projects[idx] = entry; else projects.push(entry);
              await writeProjects(projects);
              send({ status: "done" });
            } catch (err) {
              send({ status: "error", message: (err as Error).message });
            } finally {
              controller.close();
            }
          },
        });
        return new Response(stream, { headers: { "Content-Type": "application/x-ndjson", ...corsHeaders() } });
      }

      if (req.method === "GET" && url.pathname === "/health") {
        const checkFn = opts.isAvailableFn ?? (async () => {
          const s = await checkSystemRequirements();
          return { ollamaRunning: s.ollamaRunning, ollamaModel: s.ollamaModel };
        });
        const { ollamaRunning, ollamaModel } = await checkFn();
        return Response.json({ ok: true, backend: ollamaRunning ? "ollama" : "none", ollamaRunning, ollamaModel });
      }

      if (req.method === "GET" && url.pathname === "/setup/status") {
        try {
          const status = await checkSystemRequirements();
          return Response.json(status);
        } catch {
          return Response.json({ ok: false, error: "System check failed" }, { status: 500 });
        }
      }

      if (req.method === "POST" && url.pathname === "/setup/ollama") {
        const stream = new ReadableStream({
          async start(controller) {
            const send = (obj: unknown) =>
              controller.enqueue(new TextEncoder().encode(JSON.stringify(obj) + "\n"));
            try {
              send({ status: "pulling", model: OLLAMA_MODEL });
              const res = await fetch(`${OLLAMA_BASE_URL}/api/pull`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ name: OLLAMA_MODEL, stream: true }),
              });
              if (!res.body) {
                send({ status: "error", message: "No response body from Ollama pull endpoint" });
                return;
              }
              const reader = res.body.getReader();
              const decoder = new TextDecoder();
              while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                for (const line of decoder.decode(value).split("\n").filter(Boolean)) {
                  try { send(JSON.parse(line)); } catch { /* skip malformed */ }
                }
              }
              send({ status: "done" });
            } catch (err) {
              send({ status: "error", message: (err as Error).message });
            } finally {
              controller.close();
            }
          },
        });
        return new Response(stream, { headers: { "Content-Type": "application/x-ndjson" } });
      }

      if (req.method === "POST" && url.pathname === "/analyze") {
        let body: { filePath?: string; content?: string; instructions?: Record<string, unknown> };
        try {
          body = await req.json() as typeof body;
        } catch {
          return Response.json({ ok: false, error: "Invalid JSON" }, { status: 400 });
        }
        if (!body.filePath || typeof body.content !== "string") {
          return Response.json({ ok: false, error: "filePath and content are required" }, { status: 400 });
        }
        try {
          const backendFn = opts.ollamaBackendFn ?? (() => new OllamaBackend());
          const ollama = backendFn();
          await ollama.init();
          const available = await ollama.isAvailable();
          if (!available) {
            return Response.json(makeFallbackAnalysis(body.filePath));
          }
          const analysis = await ollama.extract(body.content, {
            ...(body.instructions ?? {}),
            filePath: body.filePath,
          });
          return Response.json(analysis);
        } catch {
          return Response.json({ ok: false, error: "Extraction failed" }, { status: 500 });
        }
      }

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  const s = server as { stop: () => void; port: number };
  return { stop: () => s.stop(), port: s.port };
}

export async function serve(repoPath: string, opts: { port?: number }): Promise<void> {
  const port = opts.port ?? parseInt(process.env.SENSEI_PORT ?? "7744", 10);

  // Load repoId from config
  let repoId: string | undefined;
  try {
    const config = await loadSenseiConfig(repoPath).catch(() => null);
    if (config?.repo_id) repoId = config.repo_id;
  } catch { /* ignore */ }

  intro("sensei serve");
  log.info(`Listening on :${port}`);
  if (repoId) log.info(`Serving project: ${repoId}`);

  await createReportServer({ port, repoId, repoPath });

  // Start graph watcher
  try {
    const config = await loadSenseiConfig(repoPath).catch(() => null);
    if (config?.repo_id) {
      const repoId = config.repo_id;
      const projectName = repoId;
      const watcher = await watchRepo({
        repoPath,
        repoId,
        project: projectName,
        onUpdate: (r) => {
          if (r.added + r.updated + r.removed > 0) {
            log.info(`Graph updated: +${r.added} added, ~${r.updated} updated, -${r.removed} removed (${r.durationMs}ms)`);
          }
        },
      });
      // Clean up watcher on SIGINT/SIGTERM
      process.once("SIGINT", () => watcher.stop().catch(() => {}));
      process.once("SIGTERM", () => watcher.stop().catch(() => {}));
      log.success(`Graph watcher started (repo: ${repoId})`);
    } else {
      log.warn("No .sensei/config.yaml found — graph watcher not started. Run sensei init first.");
    }
  } catch (err) {
    log.warn(`Graph watcher failed to start: ${err instanceof Error ? err.message : String(err)}`);
  }

  // Keep process alive
  await new Promise(() => {});
}
