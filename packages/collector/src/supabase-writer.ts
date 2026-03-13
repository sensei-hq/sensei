import type { SupabaseClient } from "@supabase/supabase-js";

export interface SupabaseEvent {
  user_uuid: string;
  session_id: string | null;
  repo_id: string | null;
  phase: "pre" | "post";
  tool: string;
  project_path: string;
  input: Record<string, unknown> | null;
  ts: Date;
  seq?: number | null;
  duration_ms?: number | null;
  success?: boolean | null;
  error?: string | null;
}

/** Write a single event to sensei.events. Logs and swallows errors — never throws.
 *  The client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function writeEventToSupabase(
  client: SupabaseClient,
  event: SupabaseEvent,
): Promise<void> {
  const { error } = await client
    .from("events")
    .insert({
      user_uuid:    event.user_uuid,
      session_id:   event.session_id,
      repo_id:      event.repo_id,
      phase:        event.phase,
      tool:         event.tool,
      project_path: event.project_path,
      input:        event.input,
      ts:           event.ts.toISOString(),
      seq:          event.seq ?? null,
      duration_ms:  event.duration_ms ?? null,
      success:      event.success ?? null,
      error:        event.error ?? null,
    });
  if (error) {
    console.error("[collector] Supabase write error:", error.message);
  }
}
