Root causes found

  1. Missing vitePreprocess() in svelte.config.js
  Without it, TypeScript in <script lang="ts"> blocks is not preprocessed.
  invoke<string>(...) reaches the browser as-is, and the browser evaluates
  invoke < string > (...) — string is an undefined JS identifier. Add:
  import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
  const config = { preprocess: vitePreprocess(), kit: { ... } };

  2. BrowserPageAdapter.textContent(selector) blocks until the element exists
  If you call it on an element that's conditionally rendered and currently
  absent, it waits the full 30s default timeout. Use locator().count() to check
  existence without blocking, then tauriPage.playwrightPage for assertions that
  need auto-waiting or conditional elements.

  3. webServer must use npx vite, not bunx vite
  Bun's child_process polyfill doesn't support Playwright's subprocess spawning.
   This causes the dev server to never start.

  4. IPC mock handlers — TypeScript is safe in Bun
  Bun DOES strip TypeScript from function.toString() output. as { name?: string
  }, : string, etc. are all correctly stripped. The current app's fixture syntax
   is fine on this front.
