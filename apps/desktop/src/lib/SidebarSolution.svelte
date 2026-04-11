<script lang="ts">
  import { page } from '$app/stores';
  import type { Solution } from './types.js';

  let { solution, expanded = false, onToggle }: {
    solution: Solution;
    expanded: boolean;
    onToggle: () => void;
  } = $props();

  const subLinks = $derived([
    { label: 'Overview',  href: `/s/${solution.id}`,          icon: 'i-solar-home-2-bold-duotone' },
    { label: 'Repos',     href: `/s/${solution.id}/repos`,    icon: 'i-solar-folder-with-files-bold-duotone' },
    { label: 'Sessions',  href: `/s/${solution.id}/sessions`, icon: 'i-solar-history-bold-duotone' },
  ]);

  const isActive = $derived($page.url.pathname.startsWith(`/s/${solution.id}`));

  const roleIcon: Record<string, string> = {
    backend: 'i-solar-server-bold-duotone',
    frontend: 'i-solar-monitor-bold-duotone',
    mobile: 'i-solar-smartphone-bold-duotone',
    library: 'i-solar-box-bold-duotone',
    infra: 'i-solar-cloud-bold-duotone',
    docs: 'i-solar-document-text-bold-duotone',
  };
</script>

<div class="space-y-0.5">
  <button
    onclick={onToggle}
    class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
           {isActive ? 'bg-primary-z2 text-primary-z7 font-medium' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
  >
    <span class="shrink-0 h-5 w-5 rounded bg-primary-z3 text-[10px] font-bold text-primary-z7 flex items-center justify-center">
      {solution.name.charAt(0).toUpperCase()}
    </span>
    <span class="flex-1 text-left truncate">{solution.name}</span>
    <span class="text-[10px] text-surface-z4">{solution.repos.length}</span>
    <span class="i-solar-alt-arrow-{expanded ? 'up' : 'down'}-bold-duotone text-[10px] opacity-50"></span>
  </button>

  {#if expanded}
    <div class="pl-5 space-y-0.5">
      {#each subLinks as link}
        <a
          href={link.href}
          class="flex items-center gap-2 rounded-md px-2 py-1 text-xs no-drag transition-colors
                 {$page.url.pathname === link.href
                   ? 'bg-primary-z1 text-primary-z7 font-medium'
                   : 'text-surface-z4 hover:bg-surface-z3/40 hover:text-surface-z6'}"
        >
          <span class="text-xs {link.icon}"></span>
          {link.label}
        </a>
      {/each}
    </div>
  {/if}
</div>
