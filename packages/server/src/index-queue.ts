import { Database } from "bun:sqlite";
import { join } from "path";
import { homedir } from "os";
import { mkdir } from "fs/promises";
import { cpus } from "os";

export type JobStatus = "pending" | "running" | "done" | "failed";

export interface IndexJob {
  id: number;
  repoId: string;
  repoPath: string;
  status: JobStatus;
  attempts: number;
  createdAt: string;
  updatedAt: string;
  error: string | null;
}

const QUEUE_DB = join(homedir(), ".sensei", "queue.db");
const MAX_ATTEMPTS = 3;

export class IndexQueue {
  private db: Database;

  constructor(dbPath = QUEUE_DB) {
    this.db = new Database(dbPath);
    this.db.run(`PRAGMA journal_mode=WAL`);
    this.db.run(`
      CREATE TABLE IF NOT EXISTS index_jobs (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        repo_id    TEXT    NOT NULL UNIQUE,
        repo_path  TEXT    NOT NULL,
        status     TEXT    NOT NULL DEFAULT 'pending',
        attempts   INTEGER NOT NULL DEFAULT 0,
        created_at TEXT    NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT    NOT NULL DEFAULT (datetime('now')),
        error      TEXT
      )
    `);
    // Crash recovery: any jobs left in 'running' state were interrupted — re-queue them.
    this.db.run(`UPDATE index_jobs SET status = 'pending', updated_at = datetime('now') WHERE status = 'running'`);
  }

  /** Add or re-queue a repo. If already done, re-queues (forces re-index). If pending/running, no-op. */
  enqueue(repoId: string, repoPath: string, force = false): void {
    if (force) {
      this.db.run(`
        INSERT INTO index_jobs (repo_id, repo_path, status, attempts)
        VALUES (?, ?, 'pending', 0)
        ON CONFLICT(repo_id) DO UPDATE SET
          repo_path  = excluded.repo_path,
          status     = 'pending',
          attempts   = 0,
          error      = NULL,
          updated_at = datetime('now')
      `, [repoId, repoPath]);
    } else {
      // Only enqueue if not already pending/running/done
      this.db.run(`
        INSERT INTO index_jobs (repo_id, repo_path, status, attempts)
        VALUES (?, ?, 'pending', 0)
        ON CONFLICT(repo_id) DO UPDATE SET
          repo_path  = excluded.repo_path,
          status     = CASE WHEN status IN ('failed') THEN 'pending' ELSE status END,
          updated_at = datetime('now')
      `, [repoId, repoPath]);
    }
  }

  /** Pull the next pending job and atomically mark it running. Returns null if queue is empty. */
  next(): IndexJob | null {
    const job = this.db.query<IndexJob, [number]>(`
      SELECT id, repo_id as repoId, repo_path as repoPath, status, attempts, created_at as createdAt, updated_at as updatedAt, error
      FROM index_jobs
      WHERE status = 'pending' AND attempts < ?
      ORDER BY created_at ASC
      LIMIT 1
    `).get(MAX_ATTEMPTS);
    if (!job) return null;
    this.db.run(
      `UPDATE index_jobs SET status = 'running', attempts = attempts + 1, updated_at = datetime('now') WHERE id = ?`,
      [job.id]
    );
    return job;
  }

  markDone(id: number): void {
    this.db.run(`UPDATE index_jobs SET status = 'done', error = NULL, updated_at = datetime('now') WHERE id = ?`, [id]);
  }

  markFailed(id: number, error: string): void {
    // If attempts < MAX_ATTEMPTS, put back to pending for retry; otherwise permanently failed.
    this.db.run(`
      UPDATE index_jobs SET
        status     = CASE WHEN attempts >= ? THEN 'failed' ELSE 'pending' END,
        error      = ?,
        updated_at = datetime('now')
      WHERE id = ?
    `, [MAX_ATTEMPTS, error, id]);
  }

  /** All active (pending + running) jobs — used to populate /health response. */
  active(): IndexJob[] {
    return this.db.query<IndexJob, []>(`
      SELECT id, repo_id as repoId, repo_path as repoPath, status, attempts, created_at as createdAt, updated_at as updatedAt, error
      FROM index_jobs WHERE status IN ('pending', 'running')
      ORDER BY created_at ASC
    `).all();
  }

  /** Check if a specific repo has a pending or running job. */
  isActive(repoId: string): boolean {
    const row = this.db.query<{ cnt: number }, [string]>(
      `SELECT COUNT(*) as cnt FROM index_jobs WHERE repo_id = ? AND status IN ('pending', 'running')`
    ).get(repoId);
    return (row?.cnt ?? 0) > 0;
  }

  close(): void {
    this.db.close();
  }
}

export class WorkerPool {
  private running = 0;
  private readonly concurrency: number;
  private timer?: ReturnType<typeof setInterval>;

  constructor(
    private readonly queue: IndexQueue,
    private readonly indexFn: (repoId: string, repoPath: string) => Promise<void>,
    private readonly onUpdate: (job: IndexJob, status: "done" | "failed", error?: string) => void,
    concurrency?: number,
  ) {
    this.concurrency = concurrency ?? Math.min(4, cpus().length);
  }

  start(): void {
    this.tick();
    this.timer = setInterval(() => this.tick(), 2000);
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer);
  }

  /** Drain pending jobs up to concurrency limit. */
  private tick(): void {
    while (this.running < this.concurrency) {
      const job = this.queue.next();
      if (!job) break;
      this.running++;
      this.process(job).finally(() => {
        this.running--;
        // Immediately check for more work after a job finishes.
        this.tick();
      });
    }
  }

  private async process(job: IndexJob): Promise<void> {
    try {
      await this.indexFn(job.repoId, job.repoPath);
      this.queue.markDone(job.id);
      this.onUpdate(job, "done");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      this.queue.markFailed(job.id, msg);
      this.onUpdate(job, "failed", msg);
    }
  }
}
