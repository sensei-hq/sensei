<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';
  import { page } from '$app/state';
  import { base } from '$app/paths';

  let { children } = $props();

  const GITHUB = 'https://github.com/mizukisu/sensei';
  let isHome = $derived(page.url.pathname === `${base}/` || page.url.pathname === base);
</script>

<svelte:head>
  <title>Sensei</title>
  <meta name="description" content="AI Development Intelligence — capture how your team uses AI and feed that intelligence back into your workflow." />
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<div class="min-h-screen overflow-y-auto bg-surface-z1 text-surface-z8">

  <!-- Nav -->
  <nav class="sticky top-0 z-50 flex items-center justify-between border-b border-surface-z0 bg-surface-z1/95 px-8 h-14">
    <a href="{base}/" class="flex items-center gap-2 font-bold text-base select-none no-underline text-surface-z8">
      <span class="text-lg">⬡</span>
      <span>sensei</span>
    </a>
    <div class="flex items-center gap-5 text-sm">
      <a href="{base}/setup"
         class="{page.url.pathname.includes('/setup') ? 'text-primary-z6 font-semibold' : 'text-surface-z5 hover:text-surface-z8'} transition-colors">Setup</a>
      <a href="{base}/usage"
         class="{page.url.pathname.includes('/usage') ? 'text-primary-z6 font-semibold' : 'text-surface-z5 hover:text-surface-z8'} transition-colors">Usage</a>
      {#if isHome}
        <a href="#features" class="text-surface-z5 hover:text-surface-z8 transition-colors">Features</a>
        <a href="#sponsor" class="text-surface-z5 hover:text-surface-z8 transition-colors">Sponsor</a>
      {/if}
      <a href={GITHUB} target="_blank" rel="noopener"
         class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-xs font-medium hover:bg-surface-z3 transition-colors">
        <span class="i-solar-github-bold-duotone"></span> GitHub
      </a>
    </div>
  </nav>

  {@render children()}

  <!-- Footer -->
  <div class="border-t border-surface-z0 py-8 text-center text-xs text-surface-z4">
    <a href="{base}/" class="hover:text-surface-z6 transition-colors font-bold text-surface-z6">⬡ sensei</a>
    <span class="mx-2">·</span>
    <a href="{base}/setup" class="hover:text-surface-z6 transition-colors">Setup</a>
    <span class="mx-2">·</span>
    <a href="{base}/usage" class="hover:text-surface-z6 transition-colors">Usage</a>
    <span class="mx-2">·</span>
    <a href={GITHUB} target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">GitHub</a>
  </div>

</div>
