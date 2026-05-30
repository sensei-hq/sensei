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

// Daemon port: 7745 in dev/debug builds, 7744 in production.
// Injected at build time so the frontend default is always correct.
const isDev = !!process.env.TAURI_ENV_DEBUG || process.env.NODE_ENV !== 'production';
const daemonPort = isDev ? 7745 : 7744;
// `senseiNamespace` keeps localStorage / sessionStorage keys isolated
// between dev (sensei-dev) and prod (sensei) installs, mirroring the
// daemon dir layout (~/.sensei-dev vs ~/.sensei) and the brew formula
// labels (sensei-dev vs sensei). Without this, running dev and prod
// side-by-side would have them stomping each other's setup-complete
// flag, port cache, app-version cache, etc.
const senseiNamespace = isDev ? 'sensei-dev' : 'sensei';

// Health-bypass is decided at RUNTIME via `window.__TAURI__` (set by
// Tauri's pre-injected bootstrap script before any user JS runs). No
// build-time constant, no env var. See `health-cache.ts::isHealthBypass`.

// App version: read from package.json so reroute can compare against the
// `sensei:app-version` localStorage flag (set by the updater pre-restart).
// Avoids triggering the upgrade flow when the stored version matches the
// running binary — i.e. upgrade has already been completed.
import { readFileSync } from 'node:fs';
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf-8')) as { version: string };

export default defineConfig({
  plugins: [webkitNodeReexportFix(), UnoCSS(), sveltekit()],

  // Force postcss transformer over lightningcss. Vite 8 picks lightningcss
  // by default when `build.target` is set, which makes it choke on the
  // `@apply` directives in vendored Rokkit theme CSS (UnoCSS's
  // transformerDirectives expands @apply for source files it scans, but
  // not for CSS @import-ed from node_modules). postcss + esbuild handle
  // unknown at-rules silently; we get a clean build either way at the
  // cost of a slightly slower CSS pipeline (postcss is ~ms not ns).
  css: {
    transformer: 'postcss',
  },

  define: {
    __SENSEI_DEFAULT_PORT__: JSON.stringify(daemonPort),
    __SENSEI_APP_VERSION__: JSON.stringify(pkg.version),
    __SENSEI_NAMESPACE__: JSON.stringify(senseiNamespace),
  },

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

  // Vite 8 splits config per build environment (client, ssr). Setting
  // `build.cssMinify` at the top level applies only to `client`; the SSR
  // environment defaults to `"lightningcss"`, which then warns on every
  // `@apply` directive in vendored Rokkit theme CSS (`@apply text-warning`,
  // `@apply bg-primary`, …). UnoCSS's transformerDirectives expands @apply
  // for sources it scans, but not for CSS @import-ed from node_modules.
  // Setting cssMinify to "esbuild" in BOTH environments routes through
  // esbuild's CSS minifier, which passes unknown at-rules through silently
  // — same functional output, no console noise.
  environments: {
    client: { build: { cssMinify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false } },
    ssr:    { build: { cssMinify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false } },
  },

  optimizeDeps: {
    exclude: ['@rokkit/ui', '@rokkit/states', '@rokkit/actions', '@rokkit/core', '@rokkit/app'],
  },
});
