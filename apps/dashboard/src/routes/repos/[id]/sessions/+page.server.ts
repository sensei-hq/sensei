// apps/dashboard/src/routes/repos/[id]/sessions/+page.server.ts
import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: sessions } = await db
    .from('sessions')
    .select('id,status,last_heartbeat,created_at')
    .eq('repo_id', params.id)
    .order('created_at', { ascending: false })
    .limit(50);

  // For each session, fetch its snapshots and memory items
  const sessionIds = ((sessions ?? []) as Array<{ id: string }>).map(s => s.id);

  const { data: snapshots } = sessionIds.length > 0
    ? await db
        .from('snapshots')
        .select('id,session_id,kind,progress_summary,created_at')
        .in('session_id', sessionIds)
        .order('created_at', { ascending: false })
    : { data: [] };

  const { data: memoryItems } = sessionIds.length > 0
    ? await db
        .from('memory_items')
        .select('id,session_id,type,title,status')
        .in('session_id', sessionIds)
        .order('created_at', { ascending: false })
    : { data: [] };

  const snapshotsBySession = new Map<string, typeof snapshots>();
  for (const snap of (snapshots ?? []) as Array<{ session_id: string } & Record<string, unknown>>) {
    const arr = snapshotsBySession.get(snap.session_id) ?? [];
    arr.push(snap as any);
    snapshotsBySession.set(snap.session_id, arr);
  }

  const memoryBySession = new Map<string, typeof memoryItems>();
  for (const mem of (memoryItems ?? []) as Array<{ session_id: string | null } & Record<string, unknown>>) {
    if (!mem.session_id) continue;
    const arr = memoryBySession.get(mem.session_id) ?? [];
    arr.push(mem as any);
    memoryBySession.set(mem.session_id, arr);
  }

  return {
    repo: repo as { id: string; name: string },
    sessions: ((sessions ?? []) as Array<Record<string, unknown>>).map(s => ({
      id: s.id as string,
      status: s.status as string,
      lastHeartbeat: s.last_heartbeat as string,
      createdAt: s.created_at as string,
      snapshots: (snapshotsBySession.get(s.id as string) ?? []).map((sn: any) => ({
        id: sn.id as string,
        kind: sn.kind as string,
        progressSummary: sn.progress_summary as string,
        createdAt: sn.created_at as string,
      })),
      memoryItems: (memoryBySession.get(s.id as string) ?? []).map((m: any) => ({
        id: m.id as string,
        type: m.type as string,
        title: m.title as string,
        status: m.status as string,
      })),
    })),
  };
};
