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
const adapter = onCloudflare ? adapterCloudflare() : adapterAuto();

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter
	},
	vitePlugin: {
		dynamicCompileOptions: ({ filename }) =>
			filename.includes('node_modules') ? undefined : { runes: true }
	}
};

export default config;
