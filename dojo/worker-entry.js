// The deployed Worker's entry (#308 R5b, D44): SvelteKit's generated worker for
// every request, plus a `scheduled` handler for the Cron Trigger in
// wrangler.jsonc.
//
// WHY A WRAPPER. @sveltejs/adapter-cloudflare (7.x) generates a fetch-only worker
// and writes it to whatever `main` its wrangler config names, so the adapter is
// given its own config (wrangler.sveltekit.jsonc, svelte.config.js) and the
// deployed config (wrangler.jsonc) points `main` here. The scheduled handler
// holds no logic of its own: it dispatches an in-process request to the
// SvelteKit route /v1/internal/azure/member-sync, authenticated by the server
// secret DOJO_CRON_SECRET, so the job lives in ordinary, tested server code
// (src/lib/server/azure-member-sync.ts). A failed run throws, so it shows as a
// failed invocation in the Worker's Cron Events and logs — never silently.
import app from './.svelte-kit/cloudflare/_worker.js';

export default {
	fetch: app.fetch,

	/** @param {{ cron: string }} controller @param {Record<string, string>} env @param {{ waitUntil(p: Promise<unknown>): void }} ctx */
	async scheduled(controller, env, ctx) {
		const secret = env.DOJO_CRON_SECRET;
		if (!secret) throw new Error('azure member sync: DOJO_CRON_SECRET is not set — the scheduled member read did not run');
		const res = await app.fetch(
			new Request('https://dojo.internal/v1/internal/azure/member-sync', {
				method: 'POST',
				headers: { authorization: `Bearer ${secret}` }
			}),
			env,
			ctx
		);
		const body = await res.text();
		if (!res.ok) throw new Error(`azure member sync (${controller.cron}) failed: ${res.status} ${body.slice(0, 500)}`);
		console.log(`azure member sync (${controller.cron}): ${body.slice(0, 2000)}`);
	}
};
