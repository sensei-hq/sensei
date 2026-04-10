import { Database } from "bun:sqlite";
import { randomUUID } from "crypto";
import { join } from "path";
import { homedir } from "os";
import { mkdirSync, existsSync } from "fs";

export interface ActivitySession {
  id: string;
  repoId: string;
  task: string;
  startedAt: string;
  completedAt?: string;
  outcome?: "completed" | "partial" | "blocked";
  summary?: string;
  cost?: number;
  tokensIn?: number;
  tokensOut?: number;
}

export interface ActivityAction {
  id: string;
  sessionId: string;
  type: string; // 'edit' | 'create' | 'delete' | 'search' | 'run' | 'read'
  description: string;
  filesAffected?: string[];
  timestamp: string;
}

export interface Decision {
  id: string;
  repoId: string;
  text: string;
  context: string;
  timestamp: string;
  tags?: string[];
}

export interface BacklogItem {
  id: string;
  repoId: string;
  title: string;
  description?: string;
  status: "open" | "in_progress" | "done";
  priority: "high" | "medium" | "low";
  createdAt: string;
  updatedAt: string;
}

const instances = new Map<string, ActivityLog>();

export function getActivityLog(repoId: string): ActivityLog {
  if (!instances.has(repoId)) {
    instances.set(repoId, new ActivityLog(repoId));
  }
  return instances.get(repoId)!;
}

export class ActivityLog {
  private db: Database;
  private repoId: string;

  constructor(repoId: string) {
    this.repoId = repoId;
    const dir = join(homedir(), ".sensei", "projects", repoId);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
    const dbPath = join(dir, "activity.db");
    this.db = new Database(dbPath);
    this.initSchema();
  }

  private initSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        task TEXT NOT NULL,
        started_at TEXT NOT NULL,
        completed_at TEXT,
        outcome TEXT,
        summary TEXT,
        cost REAL,
        tokens_in INTEGER,
        tokens_out INTEGER
      );

      CREATE TABLE IF NOT EXISTS actions (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        type TEXT NOT NULL,
        description TEXT NOT NULL,
        files_affected TEXT,
        timestamp TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS decisions (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        text TEXT NOT NULL,
        context TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        tags TEXT
      );

      CREATE TABLE IF NOT EXISTS backlog (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        title TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL DEFAULT 'open',
        priority TEXT NOT NULL DEFAULT 'medium',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS snapshots (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        repo_id TEXT NOT NULL,
        progress_summary TEXT NOT NULL,
        next_step_hint TEXT,
        in_flight_files TEXT,
        completed_steps TEXT,
        diff_stat_summary TEXT,
        created_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS context_packs (
        id TEXT PRIMARY KEY,
        session_id TEXT,
        repo_id TEXT NOT NULL,
        task TEXT NOT NULL,
        total_tokens INTEGER NOT NULL DEFAULT 0,
        model_id TEXT,
        created_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS pattern_usages (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        session_id TEXT,
        pattern_name TEXT NOT NULL,
        used_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS tool_calls (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        session_id TEXT,
        tool TEXT NOT NULL,
        phase TEXT NOT NULL,
        success INTEGER,
        duration_ms INTEGER,
        input TEXT,
        error TEXT,
        ts TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS api_requests (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        session_id TEXT,
        prompt_id TEXT,
        input_tokens INTEGER NOT NULL DEFAULT 0,
        output_tokens INTEGER NOT NULL DEFAULT 0,
        cache_read_tokens INTEGER NOT NULL DEFAULT 0,
        cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
        cost_usd REAL NOT NULL DEFAULT 0,
        duration_ms INTEGER,
        model TEXT,
        recorded_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS lib_docs (
        id TEXT PRIMARY KEY,
        repo_id TEXT NOT NULL,
        lib_name TEXT NOT NULL,
        title TEXT NOT NULL,
        url TEXT,
        local_path TEXT,
        summary TEXT NOT NULL DEFAULT '',
        content TEXT,
        source_type TEXT NOT NULL,
        component TEXT,
        indexed_at TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS lib_docs_repo_lib ON lib_docs(repo_id, lib_name);
    `);
  }

  logSession(session: Omit<ActivitySession, "id">): string {
    const id = randomUUID();
    this.db
      .prepare(
        `INSERT INTO sessions (id, repo_id, task, started_at, completed_at, outcome, summary, cost, tokens_in, tokens_out)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        session.repoId,
        session.task,
        session.startedAt,
        session.completedAt ?? null,
        session.outcome ?? null,
        session.summary ?? null,
        session.cost ?? null,
        session.tokensIn ?? null,
        session.tokensOut ?? null,
      ]);
    return id;
  }

  updateSession(id: string, patch: Partial<ActivitySession>): void {
    const fields: string[] = [];
    const values: unknown[] = [];

    if (patch.completedAt !== undefined) { fields.push("completed_at = ?"); values.push(patch.completedAt); }
    if (patch.outcome !== undefined) { fields.push("outcome = ?"); values.push(patch.outcome); }
    if (patch.summary !== undefined) { fields.push("summary = ?"); values.push(patch.summary); }
    if (patch.cost !== undefined) { fields.push("cost = ?"); values.push(patch.cost); }
    if (patch.tokensIn !== undefined) { fields.push("tokens_in = ?"); values.push(patch.tokensIn); }
    if (patch.tokensOut !== undefined) { fields.push("tokens_out = ?"); values.push(patch.tokensOut); }
    if (patch.task !== undefined) { fields.push("task = ?"); values.push(patch.task); }

    if (fields.length === 0) return;
    values.push(id);
    this.db.prepare(`UPDATE sessions SET ${fields.join(", ")} WHERE id = ?`).run(values);
  }

  logAction(action: Omit<ActivityAction, "id" | "timestamp">): void {
    const id = randomUUID();
    const timestamp = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO actions (id, session_id, type, description, files_affected, timestamp)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        action.sessionId,
        action.type,
        action.description,
        action.filesAffected ? JSON.stringify(action.filesAffected) : null,
        timestamp,
      ]);
  }

  logDecision(decision: Omit<Decision, "id" | "timestamp">): void {
    const id = randomUUID();
    const timestamp = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO decisions (id, repo_id, text, context, timestamp, tags)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        decision.repoId,
        decision.text,
        decision.context,
        timestamp,
        decision.tags ? JSON.stringify(decision.tags) : null,
      ]);
  }

  addBacklogItem(item: Omit<BacklogItem, "id" | "createdAt" | "updatedAt">): string {
    const id = randomUUID();
    const now = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO backlog (id, repo_id, title, description, status, priority, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        item.repoId,
        item.title,
        item.description ?? null,
        item.status,
        item.priority,
        now,
        now,
      ]);
    return id;
  }

  updateBacklogItem(id: string, patch: Partial<BacklogItem>): void {
    const fields: string[] = ["updated_at = ?"];
    const values: unknown[] = [new Date().toISOString()];

    if (patch.title !== undefined) { fields.push("title = ?"); values.push(patch.title); }
    if (patch.description !== undefined) { fields.push("description = ?"); values.push(patch.description); }
    if (patch.status !== undefined) { fields.push("status = ?"); values.push(patch.status); }
    if (patch.priority !== undefined) { fields.push("priority = ?"); values.push(patch.priority); }

    values.push(id);
    this.db.prepare(`UPDATE backlog SET ${fields.join(", ")} WHERE id = ?`).run(values);
  }

  getRecentSessions(limit = 10): ActivitySession[] {
    const rows = this.db
      .prepare(
        `SELECT * FROM sessions WHERE repo_id = ? ORDER BY started_at DESC LIMIT ?`
      )
      .all([this.repoId, limit]) as Record<string, unknown>[];

    return rows.map((r) => ({
      id: r["id"] as string,
      repoId: r["repo_id"] as string,
      task: r["task"] as string,
      startedAt: r["started_at"] as string,
      completedAt: r["completed_at"] as string | undefined,
      outcome: r["outcome"] as ActivitySession["outcome"],
      summary: r["summary"] as string | undefined,
      cost: r["cost"] as number | undefined,
      tokensIn: r["tokens_in"] as number | undefined,
      tokensOut: r["tokens_out"] as number | undefined,
    }));
  }

  getOpenBacklog(): BacklogItem[] {
    const rows = this.db
      .prepare(
        `SELECT * FROM backlog WHERE repo_id = ? AND status != 'done' ORDER BY
          CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
          created_at ASC`
      )
      .all([this.repoId]) as Record<string, unknown>[];

    return rows.map((r) => ({
      id: r["id"] as string,
      repoId: r["repo_id"] as string,
      title: r["title"] as string,
      description: r["description"] as string | undefined,
      status: r["status"] as BacklogItem["status"],
      priority: r["priority"] as BacklogItem["priority"],
      createdAt: r["created_at"] as string,
      updatedAt: r["updated_at"] as string,
    }));
  }

  getRecentDecisions(limit = 10): Decision[] {
    const rows = this.db
      .prepare(
        `SELECT * FROM decisions WHERE repo_id = ? ORDER BY timestamp DESC LIMIT ?`
      )
      .all([this.repoId, limit]) as Record<string, unknown>[];

    return rows.map((r) => ({
      id: r["id"] as string,
      repoId: r["repo_id"] as string,
      text: r["text"] as string,
      context: r["context"] as string,
      timestamp: r["timestamp"] as string,
      tags: r["tags"] ? JSON.parse(r["tags"] as string) : undefined,
    }));
  }

  getBacklogById(id: string): BacklogItem | null {
    const r = this.db
      .prepare(`SELECT * FROM backlog WHERE id = ?`)
      .get([id]) as Record<string, unknown> | undefined;
    if (!r) return null;
    return {
      id: r["id"] as string,
      repoId: r["repo_id"] as string,
      title: r["title"] as string,
      description: r["description"] as string | undefined,
      status: r["status"] as BacklogItem["status"],
      priority: r["priority"] as BacklogItem["priority"],
      createdAt: r["created_at"] as string,
      updatedAt: r["updated_at"] as string,
    };
  }

  logSnapshot(snap: {
    sessionId: string;
    repoId: string;
    progressSummary: string;
    nextStepHint?: string;
    inFlightFiles?: string[];
    completedSteps?: string[];
    diffStatSummary?: string;
  }): string {
    const id = randomUUID();
    const createdAt = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO snapshots (id, session_id, repo_id, progress_summary, next_step_hint, in_flight_files, completed_steps, diff_stat_summary, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        snap.sessionId,
        snap.repoId,
        snap.progressSummary,
        snap.nextStepHint ?? null,
        snap.inFlightFiles ? JSON.stringify(snap.inFlightFiles) : null,
        snap.completedSteps ? JSON.stringify(snap.completedSteps) : null,
        snap.diffStatSummary ?? null,
        createdAt,
      ]);
    return id;
  }

  logContextPack(pack: {
    sessionId?: string;
    repoId: string;
    task: string;
    totalTokens: number;
    modelId?: string;
  }): string {
    const id = randomUUID();
    const createdAt = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO context_packs (id, session_id, repo_id, task, total_tokens, model_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id,
        pack.sessionId ?? null,
        pack.repoId,
        pack.task,
        pack.totalTokens,
        pack.modelId ?? null,
        createdAt,
      ]);
    return id;
  }

  getContextPacks(sessionId: string): Array<{ id: string; task: string; totalTokens: number; modelId: string | null; createdAt: string }> {
    const rows = this.db
      .prepare(`SELECT * FROM context_packs WHERE session_id = ? ORDER BY created_at DESC`)
      .all([sessionId]) as Record<string, unknown>[];
    return rows.map((r) => ({
      id: r["id"] as string,
      task: r["task"] as string,
      totalTokens: r["total_tokens"] as number,
      modelId: r["model_id"] as string | null,
      createdAt: r["created_at"] as string,
    }));
  }

  logPatternUse(repoId: string, sessionId: string | null, patternName: string): void {
    const id = randomUUID();
    const usedAt = new Date().toISOString();
    this.db
      .prepare(
        `INSERT INTO pattern_usages (id, repo_id, session_id, pattern_name, used_at)
         VALUES (?, ?, ?, ?, ?)`
      )
      .run([id, repoId, sessionId, patternName, usedAt]);
  }

  logToolCall(call: {
    sessionId?: string | null;
    tool: string;
    phase: "pre" | "post";
    success?: boolean | null;
    durationMs?: number | null;
    input?: string | null;
    error?: string | null;
    ts: string;
  }): void {
    const id = randomUUID();
    this.db
      .prepare(
        `INSERT INTO tool_calls (id, repo_id, session_id, tool, phase, success, duration_ms, input, error, ts)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id, this.repoId,
        call.sessionId ?? null,
        call.tool,
        call.phase,
        call.success != null ? (call.success ? 1 : 0) : null,
        call.durationMs ?? null,
        call.input ?? null,
        call.error ?? null,
        call.ts,
      ]);
  }

  logApiRequest(req: {
    sessionId?: string | null;
    promptId?: string | null;
    inputTokens: number;
    outputTokens: number;
    cacheReadTokens: number;
    cacheCreationTokens: number;
    costUsd: number;
    durationMs?: number | null;
    model?: string | null;
    recordedAt: string;
  }): void {
    const id = randomUUID();
    this.db
      .prepare(
        `INSERT INTO api_requests (id, repo_id, session_id, prompt_id, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost_usd, duration_ms, model, recorded_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run([
        id, this.repoId,
        req.sessionId ?? null,
        req.promptId ?? null,
        req.inputTokens, req.outputTokens,
        req.cacheReadTokens, req.cacheCreationTokens,
        req.costUsd,
        req.durationMs ?? null,
        req.model ?? null,
        req.recordedAt,
      ]);
  }

  getSessionsSince(since: string): Array<{ id: string; status: string; ftr_score: number | null }> {
    const rows = this.db
      .prepare(`SELECT id, outcome FROM sessions WHERE repo_id = ? AND started_at >= ? ORDER BY started_at DESC`)
      .all([this.repoId, since]) as Array<{ id: string; outcome: string | null }>;
    return rows.map((r) => {
      const ftr_score =
        r.outcome === "completed" ? 1.0 :
        r.outcome === "partial"   ? 0.5 :
        r.outcome === "blocked"   ? 0.0 : null;
      const status =
        r.outcome === "completed" ? "completed" :
        r.outcome === "blocked"   ? "abandoned" :
        r.outcome === "partial"   ? "completed" :
        "in_progress";
      return { id: r.id, status, ftr_score };
    });
  }

  getToolCallsSince(since: string): Array<{ tool: string; success: boolean | null; duration_ms: number | null; task_session_id: string | null }> {
    const rows = this.db
      .prepare(`SELECT tool, success, duration_ms, session_id FROM tool_calls WHERE repo_id = ? AND ts >= ? ORDER BY ts DESC`)
      .all([this.repoId, since]) as Array<{ tool: string; success: number | null; duration_ms: number | null; session_id: string | null }>;
    return rows.map((r) => ({
      tool: r.tool,
      success: r.success != null ? r.success === 1 : null,
      duration_ms: r.duration_ms,
      task_session_id: r.session_id,
    }));
  }

  getBashCommandsSince(since: string): string[] {
    const rows = this.db
      .prepare(`SELECT input FROM tool_calls WHERE repo_id = ? AND tool = 'Bash' AND phase = 'pre' AND input IS NOT NULL AND ts >= ?`)
      .all([this.repoId, since]) as Array<{ input: string }>;
    return rows.map((r) => {
      try {
        const parsed = JSON.parse(r.input) as { command?: string };
        return parsed.command ?? "";
      } catch { return ""; }
    }).filter(Boolean);
  }

  replaceLibDocs(
    libName: string,
    docs: Array<{
      title: string;
      url?: string;
      localPath?: string;
      summary: string;
      content?: string;
      sourceType: string;
      component?: string;
    }>,
  ): void {
    const deleteStmt = this.db.prepare(
      `DELETE FROM lib_docs WHERE repo_id = ? AND lib_name = ?`
    );
    const insertStmt = this.db.prepare(
      `INSERT INTO lib_docs (id, repo_id, lib_name, title, url, local_path, summary, content, source_type, component, indexed_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
    );
    const now = new Date().toISOString();
    const runAll = this.db.transaction(() => {
      deleteStmt.run([this.repoId, libName]);
      for (const doc of docs) {
        insertStmt.run([
          randomUUID(),
          this.repoId,
          libName,
          doc.title,
          doc.url ?? null,
          doc.localPath ?? null,
          doc.summary,
          doc.content ?? null,
          doc.sourceType,
          doc.component ?? null,
          now,
        ]);
      }
    });
    runAll();
  }

  getLibDocs(
    libName: string,
    opts?: { component?: string; query?: string; limit?: number },
  ): Array<{
    id: string;
    libName: string;
    title: string;
    url: string | null;
    localPath: string | null;
    summary: string;
    content: string | null;
    sourceType: string;
    component: string | null;
    indexedAt: string;
  }> {
    const limit = opts?.limit ?? 50;
    const conditions: string[] = ["repo_id = ?", "lib_name = ?"];
    const params: unknown[] = [this.repoId, libName];

    if (opts?.component) {
      conditions.push("component = ?");
      params.push(opts.component);
    }

    if (opts?.query) {
      conditions.push("(title LIKE ? OR content LIKE ?)");
      params.push(`%${opts.query}%`, `%${opts.query}%`);
    }

    params.push(limit);

    const rows = this.db
      .prepare(
        `SELECT id, lib_name, title, url, local_path, summary, content, source_type, component, indexed_at
         FROM lib_docs
         WHERE ${conditions.join(" AND ")}
         ORDER BY title
         LIMIT ?`
      )
      .all(params) as Record<string, unknown>[];

    return rows.map((r) => ({
      id: r["id"] as string,
      libName: r["lib_name"] as string,
      title: r["title"] as string,
      url: r["url"] as string | null,
      localPath: r["local_path"] as string | null,
      summary: r["summary"] as string,
      content: r["content"] as string | null,
      sourceType: r["source_type"] as string,
      component: r["component"] as string | null,
      indexedAt: r["indexed_at"] as string,
    }));
  }

  getStats(): { totalSessions: number; totalActions: number; openBacklog: number } {
    const totalSessions = (
      this.db
        .prepare(`SELECT COUNT(*) AS cnt FROM sessions WHERE repo_id = ?`)
        .get([this.repoId]) as Record<string, unknown>
    )["cnt"] as number;

    const totalActions = (
      this.db
        .prepare(
          `SELECT COUNT(*) AS cnt FROM actions WHERE session_id IN (SELECT id FROM sessions WHERE repo_id = ?)`
        )
        .get([this.repoId]) as Record<string, unknown>
    )["cnt"] as number;

    const openBacklog = (
      this.db
        .prepare(`SELECT COUNT(*) AS cnt FROM backlog WHERE repo_id = ? AND status != 'done'`)
        .get([this.repoId]) as Record<string, unknown>
    )["cnt"] as number;

    return { totalSessions, totalActions, openBacklog };
  }
}
