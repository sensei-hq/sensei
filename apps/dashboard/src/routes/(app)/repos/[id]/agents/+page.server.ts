import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile, stat } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { AgentSkillsManifest, AgentSkillFile } from '@sensei/shared';

type SkillStatus = 'present' | 'stale' | 'missing';

interface SkillRow extends AgentSkillFile {
  status: SkillStatus;
}

const STALE_DAYS = 7;

function computeStatus(generatedAt: string): SkillStatus {
  const ageMs = Date.now() - new Date(generatedAt).getTime();
  const ageDays = ageMs / (1000 * 60 * 60 * 24);
  return ageDays > STALE_DAYS ? 'stale' : 'present';
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  // The repos table stores the filesystem path in `local_path` (see migration 20260313000000)
  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;
  const manifestPath = join(repoPath, '.sensei', 'agent-skills.json');

  let agent: string | null = null;
  let skills: SkillRow[] = [];

  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as AgentSkillsManifest;
      agent = manifest.agent;

      for (const skillFile of manifest.skills) {
        if (!existsSync(skillFile.path)) {
          skills.push({ ...skillFile, status: 'missing' });
          continue;
        }
        // Use file mtime for accurate freshness
        const stats = await stat(skillFile.path);
        const status = computeStatus(stats.mtime.toISOString());
        skills.push({ ...skillFile, status });
      }
    } catch {
      // Malformed manifest — treat as unconfigured
    }
  }

  return {
    repo: repo as { id: string; name: string; local_path: string },
    agent,
    skills,
  };
};

export const actions: Actions = {
  regenerate: async ({ params }) => {
    const db = getDb();
    const { data: repo } = await db
      .from('repos')
      .select('local_path')
      .eq('id', params.id)
      .single();

    if (!repo) return fail(404, { error: 'Repo not found' });

    const repoPath = (repo as { local_path: string }).local_path;

    try {
      const proc = Bun.spawn(
        ['sensei', 'setup', '--agent', 'claude'],
        {
          cwd: repoPath,
          env: { ...process.env },
          stdout: 'pipe',
          stderr: 'pipe',
        }
      );

      // 60-second timeout
      const timeout = setTimeout(() => proc.kill(), 60_000);

      const exitCode = await proc.exited;
      clearTimeout(timeout);

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || 'sensei setup failed' });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei setup' });
    }

    redirect(303, `/repos/${params.id}/agents`);
  },
};
