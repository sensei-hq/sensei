<script lang="ts">
  import { page } from '$app/stores';

  let { children } = $props();

  const navItems = [
    { icon: 'i-solar-folder-with-files-bold-duotone', label: 'Projects',  href: '/projects'  },
    { icon: 'i-solar-lightbulb-bold-duotone',         label: 'Ideas',     href: '/ideas'     },
    { icon: 'i-solar-history-bold-duotone',           label: 'Sessions',  href: '/sessions'  },
    { icon: 'i-solar-graph-up-bold-duotone',          label: 'Graph',     href: '/graph'     },
    { icon: 'i-solar-box-bold-duotone',               label: 'Libraries', href: '/libraries' },
  ];
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════════ -->
  <aside class="flex w-48 shrink-0 flex-col border-r border-surface-z0/50 sidebar-vibrancy">

    <!-- Traffic light area + logo -->
    <div class="drag-region flex items-end px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-0.5 px-2 py-2 overflow-y-auto">
      <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Workspace</p>
      {#each navItems as item}
        <a
          href={item.href}
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors no-drag
                 {$page.url.pathname.startsWith(item.href)
                   ? 'bg-primary-z2 font-medium text-primary-z7'
                   : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
        >
          <span class="text-base {item.icon}"></span>
          {item.label}
        </a>
      {/each}

      <div class="pt-4">
        <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Tools</p>
        <a href="/settings#ai-editors"
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm no-drag transition-colors
                 {$page.url.pathname === '/settings' ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}">
          <span class="text-base i-solar-cpu-bold-duotone"></span>
          AI Editors
        </a>
      </div>
    </nav>

    <!-- Bottom -->
    <div class="border-t border-surface-z0/50 px-3 py-2.5">
      <a
        href="/settings"
        class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 no-drag transition-colors hover:bg-surface-z3/60 hover:text-surface-z7"
      >
        <span class="text-base i-solar-settings-minimalistic-bold-duotone"></span>
        Settings
      </a>
    </div>
  </aside>

  <!-- ══ MAIN ═════════════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col bg-surface-z1 overflow-hidden">
    <!-- Title bar drag region -->
    <div class="drag-region h-7 shrink-0 border-b border-surface-z0/30"></div>
    <main class="flex-1 overflow-hidden min-h-0">
      {@render children()}
    </main>
  </div>

</div>
