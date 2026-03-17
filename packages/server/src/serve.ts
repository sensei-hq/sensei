import { Database } from "bun:sqlite";
import { mkdir } from "fs/promises";
import { dirname } from "path";
import { intro, log } from "@clack/prompts";
import { senseiPath } from "@sensei/shared";
import { checkSystemRequirements, OLLAMA_BASE_URL, OLLAMA_MODEL } from "./model/system-check.js";
import { OllamaBackend, makeFallbackAnalysis } from "./model/ollama-backend.js";

export interface ServeOptions {
  port?: number;
  dbPath?: string;
  repoPath?: string;
  isAvailableFn?: () => Promise<{ ollamaRunning: boolean; ollamaModel: boolean }>;  // injectable for tests
  ollamaBackendFn?: () => OllamaBackend;  // injectable for tests
}

export async function createReportServer(opts: ServeOptions = {}): Promise<{ stop: () => void; port: number }> {
  const port = opts.port ?? 7744;
  const dbPath = opts.dbPath ?? senseiPath(".", "reports.db");

  await mkdir(dirname(dbPath), { recursive: true });

  const db = new Database(dbPath);
  db.run(`
    CREATE TABLE IF NOT EXISTS reports (
      id TEXT PRIMARY KEY,
      timestamp TEXT NOT NULL,
      payload TEXT NOT NULL
    )
  `);

  const server = Bun.serve({
    port,
    async fetch(req: Request) {
      const url = new URL(req.url);

      if (req.method === "GET" && url.pathname === "/health") {
        const checkFn = opts.isAvailableFn ?? (async () => {
          const s = await checkSystemRequirements();
          return { ollamaRunning: s.ollamaRunning, ollamaModel: s.ollamaModel };
        });
        const { ollamaRunning, ollamaModel } = await checkFn();
        return Response.json({ ok: true, backend: ollamaRunning ? "ollama" : "none", ollamaRunning, ollamaModel });
      }

      if (req.method === "POST" && url.pathname === "/reports") {
        try {
          const body = await req.json() as Record<string, unknown>;
          const id = (body.id as string) ?? crypto.randomUUID();
          const timestamp = (body.timestamp as string) ?? new Date().toISOString();
          db.run(
            "INSERT OR REPLACE INTO reports (id, timestamp, payload) VALUES (?, ?, ?)",
            [id, timestamp, JSON.stringify(body)],
          );
          return Response.json({ ok: true, id });
        } catch {
          return Response.json({ ok: false, error: "Invalid JSON" }, { status: 400 });
        }
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
          await ollama.init(); // no-op now; ensures future stateful backends warm up correctly
          const available = await ollama.isAvailable();
          if (!available) {
            return Response.json(makeFallbackAnalysis(body.filePath));
          }
          const analysis = await ollama.extract(body.content, {
            ...(body.instructions ?? {}),
            filePath: body.filePath,  // always wins over instructions spread
          });
          return Response.json(analysis);
        } catch (err) {
          return Response.json({ ok: false, error: "Extraction failed" }, { status: 500 });
        }
      }

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  const s = server as { stop: () => void; port: number };
  return { stop: () => s.stop(), port: s.port };
}

export async function serve(repoPath: string, opts: { port?: number; db?: string }): Promise<void> {
  const port = opts.port ?? parseInt(process.env.SENSEI_PORT ?? "7744", 10);
  const dbPath = opts.db ?? process.env.SENSEI_DB ?? senseiPath(repoPath, "reports.db");

  intro("sensei serve");
  log.info(`Listening on :${port}`);
  log.info(`Database: ${dbPath}`);

  await createReportServer({ port, dbPath });
  // Keep process alive
  await new Promise(() => {});
}
