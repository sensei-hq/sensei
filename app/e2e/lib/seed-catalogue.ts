// What AddMember's picker reads (#455): two sessions this machine has seen — a Claude
// Code session and a Cursor-hosted one with its model — and Cursor's install row, as
// detection stores it (installed, sensei configured, the isolated home's config). Each
// session has a fixed client_session_id and `on conflict do nothing`, so a re-run on a
// kept database does not double the count. Only ever on the e2e database: the daemon
// on the port must say it is reading it.
import { execFileSync } from 'child_process';
import { join } from 'path';
import { psqlBin } from './psql';
import { E2E_DB } from './seed-ticket';

export const CATALOGUE_MODEL = 'claude-sonnet-4-5';

/** The SQL, apart so a spec can pin it. `:'config'` is the Cursor config path. */
export const SEED_CATALOGUE_SQL = `
insert into activity.sessions (client_session_id, acp_id, started_at)
values ('e2e-catalogue-claude', 'claude', now())
on conflict (client_session_id) where client_session_id is not null do nothing;
insert into activity.sessions (client_session_id, acp_id, host, provider, model, started_at)
values ('e2e-catalogue-cursor', 'cursor', 'cursor', 'anthropic', '${CATALOGUE_MODEL}', now())
on conflict (client_session_id) where client_session_id is not null do nothing;
update activity.sessions set started_at = now()
 where client_session_id in ('e2e-catalogue-claude', 'e2e-catalogue-cursor');
insert into sensei.assistant_installs (catalogue_id, installed, config_path, configured, detected_at)
values ('cursor', true, :'config', true, now())
on conflict (catalogue_id) do update
   set installed = true, config_path = excluded.config_path, configured = true, detected_at = now();
select count(*) from activity.sessions
 where acp_id in ('claude', 'claude-code') and started_at > now() - interval '7 days';`;

async function refuseUnlessE2e(daemonUrl: string): Promise<void> {
  const health = (await fetch(`${daemonUrl}/health`).then((r) => r.json())) as { dbName?: string };
  if (health.dbName !== E2E_DB) throw new Error(`refusing to seed the catalogue into "${health.dbName}"`);
}

/** Seed the sessions and Cursor's install row; returns how many Claude Code sessions
 *  started this week (the number Assistants shows), whatever earlier specs left. */
export async function seedCatalogue(daemonUrl: string, home: string): Promise<number> {
  await refuseUnlessE2e(daemonUrl);
  const out = execFileSync(
    psqlBin(),
    ['-d', E2E_DB, '-v', 'ON_ERROR_STOP=1', '-v', `config=${join(home, '.cursor', 'mcp.json')}`, '-qAt'],
    { input: SEED_CATALOGUE_SQL, stdio: ['pipe', 'pipe', 'pipe'] },
  ).toString();
  const n = Number(out.trim().split('\n').pop());
  if (!Number.isInteger(n) || n < 1) throw new Error(`catalogue not seeded (psql said "${out.trim()}")`);
  return n;
}
