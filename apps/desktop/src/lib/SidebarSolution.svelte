<script lang="ts">
  import { page } from '$app/stores';
  import type { Solution } from './types.js';

  let { solution }: { solution: Solution } = $props();

  const isActive = $derived(($page.url.pathname as string).startsWith(`/s/${solution.id}`));
</script>

<a
  href="/s/{solution.id}"
  class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
         {isActive ? 'bg-primary-z2 text-primary-z7 font-medium' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
>
  <span class="shrink-0 h-5 w-5 rounded bg-primary-z3 text-[10px] font-bold text-primary-z7 flex items-center justify-center">
    {solution.name.charAt(0).toUpperCase()}
  </span>
  <span class="flex-1 text-left truncate">{solution.name}</span>
  <span class="text-[10px] text-surface-z4">{solution.repos.length}</span>
</a>
