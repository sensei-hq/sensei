import { execFileSync } from 'child_process';

/** Resolve psql: PATH first, then the homebrew locations (a spawned app env may not carry the user's PATH). */
export function psqlBin(): string {
  for (const candidate of ['psql', '/opt/homebrew/bin/psql', '/usr/local/bin/psql']) {
    try {
      execFileSync(candidate, ['--version'], { stdio: 'ignore' });
      return candidate;
    } catch { /* not here */ }
  }
  throw new Error('psql not found (PATH, /opt/homebrew/bin, /usr/local/bin)');
}
