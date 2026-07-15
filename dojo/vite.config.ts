import { kavach } from '@kavach/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';

// Mirror the kavach demo (sites/demo/vite.config.js): the `kavach()` plugin
// reads kavach.config.js and generates the virtual `$kavach/*` modules
// ($kavach/auth, $kavach/providers, …) that wire @kavach/adapter-supabase,
// @kavach/sentry (the route guard) and the supabase client. UnoCSS + presetRokkit
// supply the named-token utilities. Rokkit packages ship as JS source, so
// exclude them from dep pre-bundling like the demo does.
export default defineConfig({
	plugins: [kavach(), UnoCSS(), sveltekit()],
	optimizeDeps: {
		exclude: ['@rokkit/app', '@rokkit/ui', '@rokkit/states', '@rokkit/actions']
	}
});
