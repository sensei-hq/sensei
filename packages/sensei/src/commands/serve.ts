import { Database } from "bun:sqlite";
import { mkdir } from "fs/promises";
import { dirname } from "path";

export interface ServeOptions {
  port?: number;
  dbPath?: string;
}

export async function createReportServer(opts: ServeOptions = {}): Promise<{ stop: () => void }> {
  const port = opts.port ?? 7744;
  const dbPath = opts.dbPath ?? ".sensei/reports.db";

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
    async fetch(req) {
      const url = new URL(req.url);

      if (req.method === "GET" && url.pathname === "/health") {
        return Response.json({ ok: true });
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

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  return { stop: () => server.stop() };
}

export async function serve(repoPath: string, opts: { port?: number; db?: string }): Promise<void> {
  const port = opts.port ?? parseInt(process.env.SENSEI_PORT ?? "7744", 10);
  const dbPath = opts.db ?? process.env.SENSEI_DB ?? `${repoPath}/.sensei/reports.db`;

  console.log(`sensei serve listening on :${port}`);
  console.log(`Database: ${dbPath}`);

  await createReportServer({ port, dbPath });
  // Keep process alive
  await new Promise(() => {});
}
