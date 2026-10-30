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
    },
    typescript: {
      // The shared packages import rokkit, which each host installs; their own
      // folder has no node_modules. vite.config.ts dedupes it for the build; this
      // tells the type checker where it is — the exact names only, so a subpath
      // (`@rokkit/graph/schema`) still resolves through the package's exports.
      // Relative to .svelte-kit/, where the generated tsconfig is written.
      config: (tsconfig) => {
        const paths = (tsconfig.compilerOptions.paths ??= {});
        for (const name of ['@rokkit/ui', '@rokkit/graph']) paths[name] = [`../node_modules/${name}`];
        return tsconfig;
      },
    },
  },
  vitePlugin: {
    dynamicCompileOptions: ({ filename }) =>
      filename.includes('node_modules') ? undefined : { runes: true },
  },
};

export default config;
