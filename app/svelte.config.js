import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    adapter: adapter({
      fallback: 'index.html',   // SPA mode — Tauri serves index.html for all routes
    }),
    alias: {
      '$lib': './src/lib',
      // Shared with the dōjō (#133); vite.config.ts lets the dev server read it.
      '@sensei/glyphs': '../packages/glyphs/index.js',
      // Svelte components shared with the dōjō (#293).
      '@sensei/ui': '../packages/ui/index.js',
      // The Code map (M17), shared with the dōjō.
      '@sensei/codemap': '../packages/codemap/index.ts',
      // The shared packages import rokkit, which each host installs; resolve it
      // here for them, as their own folder has no node_modules.
      '@rokkit/ui': './node_modules/@rokkit/ui',
    },
  },
  vitePlugin: {
    dynamicCompileOptions: ({ filename }) =>
      filename.includes('node_modules') ? undefined : { runes: true },
  },
};

export default config;
