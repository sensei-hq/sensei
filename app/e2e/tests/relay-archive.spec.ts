/**
 * Relay archival end to end (#393): closing a crew moves its rows out of the
 * live schema into `relay_archive`, and the sweep purges archive rows past
 * 30 days while younger ones stay. API-driven with psql assertions (the
 * journey spec covers the screens over archived runs).
 */
import { execFileSync } from 'child_process';
import { test, expect } from '../fixtures';
import { DAEMON_URL } from '../helpers';
import { RelayCrew } from '../lib/relay-crew';
import { psqlBin } from '../lib/psql';

const psql = (sql: string): string =>
  execFileSync(psqlBin(), ['-d', 'sensei_e2e', '-Atqv', 'ON_ERROR_STOP=1', '-c', sql], {
    stdio: 'pipe',
    encoding: 'utf8',
  }).trim();

test('closing a crew archives its rows out of the live schema', async () => {
  const crew = new RelayCrew(DAEMON_URL);
  const coord = await crew.register({ role: 'coordinator', family: 'claude-code' });
  const crewId = coord.crewId;
  const out = await coord.closeCrew();
  expect(out.status).toBe('closed');

  for (const table of ['crews', 'lanes', 'leases']) {
    const col = table === 'crews' ? 'id' : 'crew_id';
    expect(psql(`select count(*) from relay.${table} where ${col} = '${crewId}'`)).toBe('0');
    expect(psql(`select count(*) from relay_archive.${table} where ${col} = '${crewId}'`)).not.toBe('0');
  }
  // Token hashes are copied: a re-close authenticates its caller lane
  // against the archive for its 200 no-op.
  expect(psql(`select count(*) from relay_archive.lanes where crew_id = '${crewId}' and token_hash is not null`)).toBe(
    '1',
  );

  // Scrub so later runs start clean.
  for (const table of ['leases', 'lanes', 'crews']) {
    const col = table === 'crews' ? 'id' : 'crew_id';
    psql(`delete from relay_archive.${table} where ${col} = '${crewId}'`);
  }
});

test('the sweep purges archive rows past 30 days and keeps younger ones', async () => {
  const oldId = '01938931-0000-7000-8000-00000000e301';
  const youngId = '01938931-0000-7000-8000-00000000e302';
  const user = '01938931-0000-7000-8000-0000000000a1';
  for (const [id, days] of [
    [oldId, 31],
    [youngId, 29],
  ]) {
    psql(
      `insert into relay_archive.runs (user_id, id, title, goal, state, archived_at) values ('${user}', '${id}', 'R', 'G', 'completed', now() - interval '${days} days') on conflict (user_id, id) do nothing`,
    );
  }
  try {
    // The lease sweep runs every 60 s and purges as it passes.
    await expect
      .poll(() => psql(`select count(*) from relay_archive.runs where id = '${oldId}'`), { timeout: 120_000 })
      .toBe('0');
    expect(psql(`select count(*) from relay_archive.runs where id = '${youngId}'`)).toBe('1');
  } finally {
    psql(`delete from relay_archive.runs where id in ('${oldId}', '${youngId}')`);
  }
});
