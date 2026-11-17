// The e2e ticket seed (#449): what it may touch, pinned without a database.
import { describe, it, expect } from 'vitest';
import { E2E_DB, SEED_TICKET_SQL } from './seed-ticket';

describe('the e2e ticket seed', () => {
  it('only the e2e database', () => {
    expect(E2E_DB).toBe('sensei_e2e');
  });

  it('a tracker no sync can reach: a host that cannot resolve, a keychain service nothing holds', () => {
    expect(SEED_TICKET_SQL).toContain("'https://e2e-tracker.invalid'");
    expect(SEED_TICKET_SQL).toContain("'sensei-e2e-tracker-none'");
  });

  it('the values are psql variables, quoted by psql; the inserted key comes back so a no-op is caught', () => {
    expect(SEED_TICKET_SQL).toContain(":'key', :'key', :'summary'");
    expect(SEED_TICKET_SQL.trim().endsWith('returning key;')).toBe(true);
  });
});
