import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/vite';
import type { Plugin } from 'vite';

const host = process.env.TAURI_DEV_HOST;

// Workaround for WebKit (WKWebView / Safari) ES module TDZ bug with Vite 8 +
// Svelte 5 (SK#15287).
//
// Root cause (confirmed via progressive error messages):
//   WebKit/JSC resolves dynamic `import()` promises before its imported modules'
//   bodies finish executing.  This means `const`/`let` bindings and import live-
//   bindings in the node file body are still in TDZ when the router first reads
//   `branch_node.node.component`.  Even a function declaration export only moves
//   the TDZ into the body of that function where the import binding lives.
//
// Two-part fix:
//  1. Transform node files to expose a var-cached copy of the component and a
//     `_wk_ready` Promise that resolves once the var is populated.  `var` is
//     hoisted to `undefined`, so the export is never in TDZ; the Promise
//     ensures the component is assigned before it is called.
//  2. Transform the `app.js` node loaders to await `_wk_ready` after the module
//     resolves, guaranteeing the var is set before SvelteKit reads `.component`.
function webkitNodeReexportFix(): Plugin {
  return {
    name: 'webkit-node-reexport-fix',

    transform(code, id) {
      // --- node files: expose a var-cached component + ready Promise ---
      if (id.includes('.svelte-kit/generated/client/nodes/')) {
        return code.replace(
          /export \{ default as (\w+) \} from (['"][^'"]+['"])/g,
          (_, name, from) =>
            `import _wk_${name}_import from ${from};` +
            `var ${name};` +
            `export var _wk_ready = Promise.resolve().then(function(){${name}=_wk_${name}_import;});` +
            `export { ${name} };`,
        );
      }

      // --- app.js: wrap every node loader to await _wk_ready ---
      // WKWebView resolves import() BEFORE the module body executes (SK#15287).
      // setTimeout(0) crosses a full event-loop iteration, guaranteeing the module
      // body has run and _wk_ready is set before we inspect it.  Used in both dev
      // and production so behaviour is identical in both environments.
      if (id.includes('.svelte-kit/generated/client/app.js')) {
        return code.replace(
          /\(\) => import\((['"][^'"]+['"])\)/g,
          (_, path) =>
            `async () => { var _m = await import(${path}); await new Promise(r => setTimeout(r, 0)); if (_m._wk_ready) await _m._wk_ready; return _m; }`,
        );
      }
    },
  };
}

export default defineConfig({
  plugins: [webkitNodeReexportFix(), UnoCSS(), sveltekit()],

  // Tauri: don't open browser, use Tauri window
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5183 } : undefined,
  },

  // Tauri expects a inlined build with no CDN
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari15',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },

  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app'],
  },
});
