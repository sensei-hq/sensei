import adapterAuto from '@sveltejs/adapter-auto';
import adapterCloudflare from '@sveltejs/adapter-cloudflare';

// Adapter selection ("keep both" — mirrors kavach's sites/learn, which deploys
// as a Cloudflare Worker at kavach.sensei-hq.com):
//   - On Cloudflare Workers Builds (auto-sets WORKERS_CI) use
//     @sveltejs/adapter-cloudflare explicitly → emits .svelte-kit/cloudflare/
//     (a _worker.js + auto-generated .assetsignore) which the committed
//     wrangler.jsonc deploys via `wrangler deploy`. Selecting it here (rather
//     than letting adapter-auto auto-install it at build time) avoids the
//     frozen-lockfile auto-install failure.
//   - Do NOT set CF_PAGES=1: it forces Pages-style output (_routes.json, no
//     .assetsignore) which breaks `wrangler deploy`. Left below only as a
//     harmless fallback.
//   - Everywhere else (local dev, CI) adapter-auto picks the right target. dojo
//     is NOT static — kavach SSR auth (hooks.server.ts, /auth/session, /data)
//     runs in the Worker.
const onCloudflare = Boolean(process.env.WORKERS_CI) || Boolean(process.env.CF_PAGES);
// The adapter builds against its own wrangler config: it writes SvelteKit's
// worker to that config's `main`, while the deployed wrangler.jsonc's `main` is
// worker-entry.js — the wrapper adding the cron's `scheduled` handler (#308 R5b, D44).
const adapter = onCloudflare ? adapterCloudflare({ config: 'wrangler.sveltekit.jsonc' }) : adapterAuto();

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter,
		alias: {
			// The kanji vocabulary shared with the desktop app (#133); dev-server.ts
			// lets the dev server read it.
			'@sensei/glyphs': '../packages/glyphs/index.js',
			'@sensei/sdlc-figures': '../packages/sdlc-figures/index.js',
			// Svelte components shared with the desktop app (#293).
			'@sensei/ui': '../packages/ui/index.js'
		},
		typescript: {
			// As in app/svelte.config.js: packages/ui imports rokkit, which each host
			// installs; its own folder has no node_modules. vite.config.ts dedupes it
			// for the build and vitest; this tells the type checker where it is — the
			// exact names only, so a subpath still resolves through the package's
			// exports. Relative to .svelte-kit/, where the generated tsconfig is written.
			config: (tsconfig) => {
				const paths = (tsconfig.compilerOptions.paths ??= {});
				for (const name of ['@rokkit/ui', '@rokkit/states', '@rokkit/core']) paths[name] = [`../node_modules/${name}`];
				return tsconfig;
			}
		}
	},
	vitePlugin: {
		dynamicCompileOptions: ({ filename }) =>
			filename.includes('node_modules') ? undefined : { runes: true }
	}
};

export default config;
