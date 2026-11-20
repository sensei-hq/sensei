import { defineConfig } from '@playwright/test';
import { DOJO_URL } from './local-dojo';

// The dōjō browser specs (#299). They run the built Worker under `wrangler dev`
// against the LOCAL Supabase only: `local-dojo.ts` reads `.dev.vars` and refuses
// any Supabase URL that is not on this machine. Port 5175 is in the local auth
// redirect allow-list (supabase/config.toml) and leaves `make dojo-local` (5173)
// alone.
//
// Run: `cd dojo && bun run test:e2e` (local Supabase up; converge it first with
// scripts/deploy-dojo-db.sh).
export default defineConfig({
	testDir: '.',
	testMatch: /.*\.e2e\.ts/,
	timeout: 120_000,
	retries: 0,
	workers: 1,
	reporter: [['list']],
	use: { baseURL: DOJO_URL, browserName: 'chromium' },
	webServer: {
		command: 'CF_PAGES=1 bun run build && bunx wrangler dev --port 5175 --ip 127.0.0.1 --local',
		url: `${DOJO_URL}/signin`,
		reuseExistingServer: true,
		timeout: 600_000
	}
});
