import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { kavach } from '@kavach/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';

// Build-time version stamp so a deploy is verifiable (dojo v{version} · {sha} in
// the console footer + GET /version). Computed here in the Node config context
// and injected via `define` — see src/lib/version.ts (the DRY accessor) and
// src/app.d.ts (ambient declarations).
const pkg = JSON.parse(
	readFileSync(new URL('./package.json', import.meta.url), 'utf8')
) as { version: string };

// Short commit sha; execFileSync (no shell, fixed argv — no injection surface)
// wrapped in try/catch so a CI/shallow-clone without git still builds.
function gitSha(): string {
	try {
		return execFileSync('git', ['rev-parse', '--short', 'HEAD'], {
			encoding: 'utf8'
		}).trim();
	} catch {
		return 'unknown';
	}
}

// Mirror the kavach demo (sites/demo/vite.config.js): the `kavach()` plugin
// reads kavach.config.js and generates the virtual `$kavach/*` modules
// ($kavach/auth, $kavach/providers, …) that wire @kavach/adapter-supabase,
// @kavach/sentry (the route guard) and the supabase client. UnoCSS + presetRokkit
// supply the named-token utilities. Rokkit packages ship as JS source, so
// exclude them from dep pre-bundling like the demo does.
export default defineConfig({
	define: {
		__DOJO_VERSION__: JSON.stringify(pkg.version),
		__DOJO_GIT_SHA__: JSON.stringify(gitSha()),
		__DOJO_BUILD_TIME__: JSON.stringify(new Date().toISOString())
	},
	plugins: [
		// Compile-time i18n (paraglide/inlang) — generates $lib/paraglide (m.* messages)
		// from messages/{locale}.json. Runs before sveltekit so the generated module is
		// present for the app graph. English-only today; add a locale via a new messages
		// file + project.inlang/settings.json. See docs/spec/dojo-screens/README.md.
		paraglideVitePlugin({ project: './project.inlang', outdir: './src/lib/paraglide' }),
		kavach(),
		UnoCSS(),
		sveltekit()
	],
	optimizeDeps: {
		exclude: ['@rokkit/app', '@rokkit/ui', '@rokkit/states', '@rokkit/actions']
	}
});
