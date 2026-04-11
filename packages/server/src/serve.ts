import { mkdir, readFile, writeFile } from "fs/promises";
import { dirname, join, relative } from "path";
import { homedir } from "os";
import { IndexQueue, WorkerPool } from "./index-queue.js";
import { intro, log } from "@clack/prompts";
import { loadSenseiConfig } from "@sensei/shared";
import { watchRepo, getOrCreateDb, indexRepo, progressPath, type IndexProgress } from "@sensei/graph-indexer";
import { checkSystemRequirements, OLLAMA_BASE_URL, OLLAMA_MODEL } from "./model/system-check.js";
import { OllamaBackend, makeFallbackAnalysis } from "./model/ollama-backend.js";
import { getActivityLog } from "./activity-log.js";
import { checkDrift, search, getLlmSpec, listExports, getFileContext } from "@sensei/tools";

const PROJECTS_FILE = join(homedir(), ".sensei", "projects.json");
const PID_FILE = join(homedir(), ".sensei", "serve.pid");

export interface ProjectEntry {
  repoId: string;
  name: string;
  path: string;
  indexedAt?: string;
  lastError?: string;
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

export async function createReportServer(opts: ServeOptions = {}): Promise<{ stop: () => void; port: number; queue: IndexQueue }> {
  const port = opts.port ?? 7744;
  const activeRepoId = opts.repoId ?? null;
  const activeRepoPath = opts.repoPath ?? null;

  await mkdir(join(homedir(), ".sensei"), { recursive: true });
  const queue = new IndexQueue();
  const pool = new WorkerPool(
    queue,
    async (repoId, repoPath) => {
      await indexRepo({ repoId, repoPath, project: repoId });
      const projects = await readProjects();
      const idx = projects.findIndex((p) => p.repoId === repoId);
      const entry: ProjectEntry = { repoId, name: repoId, path: repoPath, indexedAt: new Date().toISOString() };
      if (idx >= 0) projects[idx] = entry; else projects.push(entry);
      await writeProjects(projects);
    },
    async (job, status, error) => {
      if (status === "done") {
        log.success(`[${job.repoId}] indexed`);
      } else {
        log.warn(`[${job.repoId}] index failed (attempt ${job.attempts}/${3}): ${error}`);
        // After exhausting retries, persist the error so the UI can surface it
        if (job.attempts >= 3 && error) {
          const projects = await readProjects().catch(() => [] as ProjectEntry[]);
          const idx = projects.findIndex(p => p.repoId === job.repoId);
          if (idx >= 0) {
            projects[idx] = { ...projects[idx], lastError: error };
            await writeProjects(projects).catch(() => {});
          }
        }
      }
    },
  );
  pool.start();

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
          // Annotate each project with partiallyIndexed: true if a manifest exists
          // but the repo hasn't completed a full index yet.
          const { existsSync: fsExistsSync } = await import("fs");
          const annotated = projects.map(p => {
            const manifestFile = join(homedir(), ".sensei", "projects", p.repoId, "manifest.json");
            const partiallyIndexed = !p.indexedAt && fsExistsSync(manifestFile);
            return { ...p, partiallyIndexed };
          });
          return jsonResponse(annotated);
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

      if (req.method === "GET" && url.pathname === "/api/drift") {
        const repoPath = url.searchParams.get("repoPath") ?? activeRepoPath;
        if (!repoPath) {
          return jsonResponse({ drifted: [], summary: "repoPath required" });
        }
        try {
          const result = await checkDrift(repoPath);
          return jsonResponse(result);
        } catch (err) {
          return jsonResponse({ ok: false, error: (err as Error).message }, 500);
        }
      }

      if (req.method === "POST" && url.pathname === "/api/mcp") {
        let body: { tool: string; args?: Record<string, unknown> };
        try {
          body = await req.json() as typeof body;
        } catch {
          return jsonResponse({ ok: false, error: "Invalid JSON" }, 400);
        }
        const args = body.args ?? {};
        const repoPath = (args.repoPath as string | undefined) ?? activeRepoPath ?? "";
        try {
          let result: unknown;
          switch (body.tool) {
            case "search":
              result = await search(repoPath, args.query as string, { top: (args.limit as number | undefined) ?? 10 });
              break;
            case "get_lib_docs":
              result = await getLlmSpec(repoPath, args.section as string | undefined);
              break;
            case "list_exports":
              result = await listExports(repoPath, args.module as string | undefined);
              break;
            case "get_file_context":
              result = await getFileContext(repoPath, args.filePath as string, (args.level as string | undefined ?? "L2") as import("@sensei/shared").ResolutionLevel);
              break;
            case "check_drift":
              result = await checkDrift(repoPath);
              break;
            default:
              return jsonResponse({ ok: false, error: `Unknown tool: ${body.tool}` }, 400);
          }
          return jsonResponse({ ok: true, result });
        } catch (err) {
          return jsonResponse({ ok: false, error: (err as Error).message }, 500);
        }
      }

      if (req.method === "POST" && url.pathname === "/api/index") {
        let body: { repoId?: string; repoPath?: string; force?: boolean };
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
        queue.enqueue(repoId, repoPath, body.force);
        return jsonResponse({ ok: true, status: "queued", repoId });
      }

      if (req.method === "GET" && url.pathname === "/health") {
        const checkFn = opts.isAvailableFn ?? (async () => {
          const s = await checkSystemRequirements();
          return { ollamaRunning: s.ollamaRunning, ollamaModel: s.ollamaModel };
        });
        const { ollamaRunning, ollamaModel } = await checkFn();
        const active = queue.active();

        // Attach per-repo progress (current file, counts) for running jobs
        const progress: Record<string, IndexProgress> = {};
        await Promise.all(active.filter(j => j.status === "running").map(async j => {
          try {
            const raw = await readFile(progressPath(j.repoId), "utf-8");
            if (raw.trim()) progress[j.repoId] = JSON.parse(raw) as IndexProgress;
          } catch { /* no progress file yet */ }
        }));

        return jsonResponse({
          ok: true,
          name: "sensei",
          version: "0.1.0",
          backend: ollamaRunning ? "ollama" : "none",
          ollamaRunning,
          ollamaModel,
          indexing: active.length > 0 ? active.map(j => j.repoId) : null,
          queue: active.map(j => ({ repoId: j.repoId, status: j.status, attempts: j.attempts })),
          progress,
        });
      }

      if (req.method === "GET" && url.pathname === "/setup/status") {
        try {
          const status = await checkSystemRequirements();
          return jsonResponse(status);
        } catch {
          return jsonResponse({ ok: false, error: "System check failed" }, 500);
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
          return jsonResponse({ ok: false, error: "Invalid JSON" }, 400);
        }
        if (!body.filePath || typeof body.content !== "string") {
          return jsonResponse({ ok: false, error: "filePath and content are required" }, 400);
        }
        try {
          const backendFn = opts.ollamaBackendFn ?? (() => new OllamaBackend());
          const ollama = backendFn();
          await ollama.init();
          const available = await ollama.isAvailable();
          if (!available) {
            return jsonResponse(makeFallbackAnalysis(body.filePath));
          }
          const analysis = await ollama.extract(body.content, {
            ...(body.instructions ?? {}),
            filePath: body.filePath,
          });
          return jsonResponse(analysis);
        } catch {
          return jsonResponse({ ok: false, error: "Extraction failed" }, 500);
        }
      }

      if (req.method === "POST" && url.pathname === "/stop") {
        // Respond before killing so the client gets the 200.
        setTimeout(() => {
          pool.stop();
          queue.close();
          process.exit(0);
        }, 100);
        return jsonResponse({ ok: true });
      }

      return jsonResponse({ ok: false, error: "Not found" }, 404);
    },
  });

  const s = server as { stop: () => void; port: number };
  return {
    stop: () => { pool.stop(); queue.close(); s.stop(); },
    port: s.port,
    queue,
  };
}

export async function serve(repoPath: string, opts: { port?: number; daemon?: boolean }): Promise<void> {
  const port = opts.port ?? parseInt(process.env.SENSEI_PORT ?? "7744", 10);
  const isDaemon = opts.daemon ?? false;

  // In daemon mode use plain stdout lines; in CLI mode use clack's styled output.
  const out = {
    info:    (msg: string) => isDaemon ? console.log(`[sensei] ${msg}`) : log.info(msg),
    warn:    (msg: string) => isDaemon ? console.error(`[sensei] warn: ${msg}`) : log.warn(msg),
    success: (msg: string) => isDaemon ? console.log(`[sensei] ${msg}`) : log.success(msg),
  };

  // Load repoId from config
  let repoId: string | undefined;
  try {
    const config = await loadSenseiConfig(repoPath).catch(() => null);
    if (config?.repo_id) repoId = config.repo_id;
  } catch { /* ignore */ }

  // Write PID file so `sensei serve stop` / `senseid stop` can find us.
  await mkdir(dirname(PID_FILE), { recursive: true });
  await writeFile(PID_FILE, String(process.pid));
  const cleanPid = () => import("fs").then(fs => fs.rmSync(PID_FILE, { force: true }));
  process.once("exit", cleanPid);
  process.once("SIGINT",  () => { cleanPid(); process.exit(0); });
  process.once("SIGTERM", () => { cleanPid(); process.exit(0); });

  if (!isDaemon) intro("sensei serve");
  out.info(`Listening on :${port}`);
  if (repoId) out.info(`Serving project: ${repoId}`);

  const { queue } = await createReportServer({ port, repoId, repoPath });

  // ── Desired-state reconciliation ────────────────────────────────────────────
  // On startup, enqueue unindexed projects and start file watchers for all.
  // Indexing runs in parallel via the WorkerPool — no sequential blocking.
  async function startWatcher(pid: string, ppath: string) {
    try {
      const watcher = await watchRepo({
        repoPath: ppath, repoId: pid, project: pid,
        onUpdate: (r) => {
          if (r.added + r.updated + r.removed > 0)
            out.info(`[${pid}] graph +${r.added} ~${r.updated} -${r.removed} (${r.durationMs}ms)`);
        },
      });
      process.once("SIGINT",  () => watcher.stop().catch(() => {}));
      process.once("SIGTERM", () => watcher.stop().catch(() => {}));
    } catch (err) {
      out.warn(`[${pid}] watcher failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // Kick off asynchronously — server already accepting connections before this resolves.
  (async () => {
    const projects = await readProjects();
    if (projects.length === 0) {
      out.warn("No projects registered — import repos via the desktop app.");
      return;
    }

    // Force-enqueue ALL unindexed repos (no indexedAt). force=true resets the attempt
    // counter so previously-failed repos (attempts=3, stuck) get 3 fresh tries.
    // Also clears any stale lastError so old errors don't show after a successful re-index.
    const unindexed = projects.filter(p => !p.indexedAt);
    if (unindexed.length > 0) {
      out.info(`Queuing ${unindexed.length} unindexed project(s) for parallel indexing…`);
      const anyHadError = unindexed.some(p => p.lastError);
      if (anyHadError) {
        const updated = projects.map(p => p.lastError ? { ...p, lastError: undefined } : p);
        await writeProjects(updated).catch(() => {});
      }
      for (const p of unindexed) queue.enqueue(p.repoId, p.path, /* force */ true);
    }

    out.info(`Starting file watchers for ${projects.length} project(s)…`);
    await Promise.all(projects.map(p => startWatcher(p.repoId, p.path)));
    out.success(`Reconciliation complete. ${unindexed.length} queued, ${projects.length - unindexed.length} already indexed.`);
  })();

  // Keep process alive
  await new Promise(() => {});
}
