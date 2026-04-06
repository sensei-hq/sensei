import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const db = (supabase as any).schema('sensei');

  const { data: toolStats } = await db
    .from('events')
    .select('tool')
    .eq('phase', 'post');

  const toolCounts: Record<string, number> = {};
  for (const e of toolStats ?? []) {
    toolCounts[e.tool] = (toolCounts[e.tool] ?? 0) + 1;
  }

  const toolRows = Object.entries(toolCounts)
    .map(([tool, count]) => ({ tool, count }))
    .sort((a, b) => b.count - a.count);

  const { data: sessions } = await db
    .from('events')
    .select('session_id, ts, tool')
    .not('session_id', 'is', null)
    .order('ts', { ascending: false })
    .limit(100);

  const sessionIds = [...new Set((sessions ?? []).map((e: any) => e.session_id))].slice(0, 10);

  const allEvents = sessions ?? [];
  const sensei_tools = new Set(['load_context', 'get_llmspec', 'get_file_context', 'recommend_next']);
  const sessionsWithContext = new Set(
    allEvents.filter((e: any) => sensei_tools.has(e.tool ?? '')).map((e: any) => e.session_id)
  );
  const gapSessions = sessionIds.filter((id: any) => !sessionsWithContext.has(id));

  return { toolRows, sessionCount: sessionIds.length, gapCount: gapSessions.length };
};
