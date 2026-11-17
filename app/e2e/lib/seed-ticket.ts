// A synced Jira ticket in the e2e broker (#449): the rows tracker sync writes — a tracker
// on the first workspace and one ticket under it — so the run journey can add the ticket
// to the backlog by key. The tracker points at a host that cannot resolve and a keychain
// service nothing holds, so a sync pass that picks it up fails without reaching anything.
// Only ever on the e2e database: the daemon on the port must say it is reading it.
import { execFileSync } from 'child_process';
import { psqlBin } from './psql';

export const E2E_DB = 'sensei_e2e';

/** The SQL, apart so a spec can pin it: one tracker (kept across runs), one ticket per key. */
export const SEED_TICKET_SQL = `
with tracker as (
  insert into sensei.trackers (workspace_id, kind, base_url, account, external_scope, board_ref, secret_service)
  select id, 'jira', 'https://e2e-tracker.invalid', 'e2e@sensei.test', 'E2E', '1', 'sensei-e2e-tracker-none'
    from sensei.workspaces order by id limit 1
  on conflict (kind, base_url, board_ref) do update set account = excluded.account
  returning id)
insert into sensei.tracker_tasks (tracker_id, external_id, key, summary, issue_type)
select id, :'key', :'key', :'summary', 'Story' from tracker
returning key;`;

export async function seedJiraTicket(daemonUrl: string, key: string, summary: string): Promise<void> {
  const health = (await fetch(`${daemonUrl}/health`).then((r) => r.json())) as { dbName?: string };
  if (health.dbName !== E2E_DB) throw new Error(`refusing to seed a ticket into "${health.dbName}"`);
  // psql interpolates :'key' and :'summary' (quoted) only in a script it reads, so the SQL goes on stdin.
  const out = execFileSync(
    psqlBin(),
    ['-d', E2E_DB, '-v', 'ON_ERROR_STOP=1', '-v', `key=${key}`, '-v', `summary=${summary}`, '-qAt'],
    { input: SEED_TICKET_SQL, stdio: ['pipe', 'pipe', 'pipe'] },
  ).toString();
  // No workspace in the e2e database inserts nothing: say so here, not as a 404 later.
  if (out.trim() !== key) throw new Error(`no ticket seeded (psql said "${out.trim()}"): does ${E2E_DB} hold a workspace?`);
}
