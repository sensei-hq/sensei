import type { SupabaseClient } from "@supabase/supabase-js";
import { existsSync, readFileSync, unlinkSync } from "fs";
import { writeEventToSupabase } from "./supabase-writer.js";

interface StoredEvent {
  user_uuid?: string;
  session_id?: string;
  seq?: number | null;
  ts?: number;  // Unix milliseconds
  tool?: string;
  phase?: string;
  duration_ms?: number | null;
  success?: boolean | null;
  input?: string | null;  // JSON string
  error?: string | null;
  project_path?: string;
}

export async function drainJsonl(client: SupabaseClient, jsonlPath: string): Promise<void> {
  if (!existsSync(jsonlPath)) return;

  const content = readFileSync(jsonlPath, "utf8");
  const lines = content.split("\n").filter(l => l.trim().length > 0);

  // Parse JSON first — parse errors are benign skips
  const validEvents: StoredEvent[] = [];
  for (const line of lines) {
    try {
      validEvents.push(JSON.parse(line) as StoredEvent);
    } catch {
      console.warn("[collector] drain: skipping malformed JSONL line");
    }
  }

  // Insert each event individually — skip invalid/failed, don't abort
  try {
    for (const e of validEvents) {
      if (!e.ts || !e.tool || (e.phase !== "pre" && e.phase !== "post")) {
        console.warn("[collector] drain: skipping event with missing required fields");
        continue;
      }
      await writeEventToSupabase(client, {
        user_uuid:    e.user_uuid ?? "",
        session_id:   e.session_id ?? null,
        repo_id:      null,
        phase:        e.phase as "pre" | "post",
        tool:         e.tool,
        project_path: e.project_path ?? "",
        input:        e.input ? (() => { try { return JSON.parse(e.input!); } catch { return null; } })() : null,
        ts:           new Date(e.ts),
        seq:          e.seq ?? null,
        duration_ms:  e.duration_ms ?? null,
        success:      e.success ?? null,
        error:        e.error ?? null,
      });
    }
  } catch (err) {
    console.error("[collector] drain: unexpected error, leaving JSONL intact:", (err as Error).message);
    return;
  }

  // All events processed — delete the JSONL file
  try {
    unlinkSync(jsonlPath);
  } catch (err) {
    console.error("[collector] drain: failed to delete JSONL file:", (err as Error).message);
  }
}
