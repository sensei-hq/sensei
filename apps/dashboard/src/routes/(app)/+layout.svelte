<script lang="ts">
  import { getContext } from 'svelte';
  import { page } from '$app/stores';
  import { ThemeSwitcherToggle } from '@rokkit/app';

  let { children } = $props();

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const kavach = getContext<Record<string, any>>('kavach');

  // Sidebar state
  let sidebarOpen = $state(false);

  // Route context drives sidebar nav
  let context = $derived(
    $page.route.id?.includes('/team/') ? 'team' : 'user'
  );

  let teamName = $derived(
    ($page.data.teamName as string | undefined) ?? $page.params.teamSlug ?? ''
  );

  function closeSidebar() { sidebarOpen = false; }
</script>

<!-- Mobile backdrop -->
{#if sidebarOpen}
  <div
    class="fixed inset-0 z-20 bg-black/60 md:hidden"
    role="presentation"
    onclick={closeSidebar}
  ></div>
{/if}

<!-- App shell -->
<div class="flex h-full overflow-hidden">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════ -->
  <aside class="
    fixed inset-y-0 left-0 z-30 flex w-64 flex-col
    border-r border-surface-z0 bg-surface-z2 overflow-y-auto
    transition-transform duration-200 ease-in-out
    {sidebarOpen ? 'translate-x-0' : '-translate-x-full'}
    md:relative md:w-56 md:shrink-0 md:translate-x-0 md:transition-none
  ">
    <!-- Logo -->
    <div class="flex items-center justify-between border-b border-surface-z0 px-4 py-3.5">
      <div class="flex items-center gap-2.5">
        <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary-z6 text-sm font-bold text-white">⬡</div>
        <span class="font-bold text-surface-z8">sensei</span>
      </div>
      <button
        class="md:hidden flex h-7 w-7 items-center justify-center rounded-lg text-surface-z5 hover:bg-surface-z3 hover:text-surface-z8"
        onclick={closeSidebar}
        aria-label="Close menu"
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" class="h-4 w-4">
          <path stroke-linecap="round" d="M3 3l10 10M13 3L3 13"/>
        </svg>
      </button>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-5 px-3 py-4">

      {#if context === 'user'}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">My workspace</p>
          {#each [
            { icon: 'i-solar-home-2-bold-duotone',          label: 'Dashboard',    href: '/home'       },
            { icon: 'i-solar-folder-2-bold-duotone',        label: 'My Repos',     href: '/repos'      },
            { icon: 'i-solar-history-bold-duotone',         label: 'Sessions',     href: '/stats'      },
            { icon: 'i-solar-database-bold-duotone',        label: 'Memory Items', href: '/references' },
            { icon: 'i-solar-dollar-minimalistic-bold-duotone', label: 'Cost Tracker', href: '/benchmarks' },
          ] as item}
            <a
              href={item.href}
              onclick={closeSidebar}
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {$page.url.pathname.startsWith(item.href) ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
            >
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </a>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Local setup</p>
          {#each [
            { icon: 'i-solar-programming-bold-duotone',       label: 'CLI Config'    },
            { icon: 'i-solar-plug-circle-bold-duotone',      label: 'Plugin Status' },
            { icon: 'i-solar-server-minimalistic-bold-duotone', label: 'MCP Server' },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </button>
          {/each}
        </div>
        {#if $page.data.teams?.length > 0}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Teams</p>
          {#each $page.data.teams as team}
            <a
              href="/team/{team.slug}"
              onclick={closeSidebar}
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {$page.url.pathname.startsWith('/team/' + team.slug) ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
            >
              <span class="text-lg i-solar-users-group-two-rounded-bold-duotone"></span>
              {team.displayName}
            </a>
          {/each}
        </div>
        {/if}
        {#if $page.data.session?.user?.role === 'platform_admin'}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Platform</p>
          {#each [
            { icon: 'i-solar-global-bold-duotone',      label: 'All Tenants',     href: '/platform'            },
            { icon: 'i-solar-chart-2-bold-duotone',     label: 'Aggregate Stats', href: '/platform/analytics'  },
            { icon: 'i-solar-bell-bing-bold-duotone',   label: 'Alerts',          href: '/platform/alerts'     },
            { icon: 'i-solar-graph-up-bold-duotone',    label: 'Benchmarks',      href: '/benchmarks'          },
          ] as item}
            <a
              href={item.href}
              onclick={closeSidebar}
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {$page.url.pathname.startsWith(item.href) ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
            >
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </a>
          {/each}
        </div>
        {/if}

      {:else if context === 'team'}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">{teamName || 'Team'}</p>
          {#each [
            { icon: 'i-solar-home-2-bold-duotone',                      label: 'Overview',        active: true  },
            { icon: 'i-solar-users-group-two-rounded-bold-duotone',      label: 'Members',         active: false },
            { icon: 'i-solar-folder-2-bold-duotone',                     label: 'Repos',           active: false },
            { icon: 'i-solar-chart-2-bold-duotone',                      label: 'Analytics',       active: false },
            { icon: 'i-solar-cpu-bold-duotone',                          label: 'Skills & Config', active: false },
          ] as item}
            <button
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {item.active ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
              onclick={closeSidebar}
            >
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </button>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Settings</p>
          {#each [
            { icon: 'i-solar-key-bold-duotone',     label: 'API Keys'    },
            { icon: 'i-solar-card-bold-duotone',    label: 'Billing'     },
            { icon: 'i-solar-tuning-2-bold-duotone', label: 'Preferences' },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </button>
          {/each}
        </div>

      {:else}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Platform</p>
          {#each [
            { icon: 'i-solar-global-bold-duotone',      label: 'All Tenants',     href: '/platform',           badge: null },
            { icon: 'i-solar-chart-2-bold-duotone',     label: 'Aggregate Stats', href: '/platform/analytics', badge: null },
            { icon: 'i-solar-bell-bing-bold-duotone',   label: 'Alerts',          href: '/platform',           badge: 3    },
            { icon: 'i-solar-graph-up-bold-duotone',    label: 'Benchmarks',      href: '/benchmarks',         badge: null },
          ] as item}
            <a
              href={item.href}
              onclick={closeSidebar}
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {$page.url.pathname === item.href && !item.badge ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
            >
              <span class="text-lg {item.icon}"></span>
              <span class="flex-1 text-left">{item.label}</span>
              {#if item.badge}
                <span class="rounded-full bg-primary-z6 px-1.5 py-0.5 text-[10px] font-semibold text-white">{item.badge}</span>
              {/if}
            </a>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">System</p>
          {#each [
            { icon: 'i-solar-settings-bold-duotone',        label: 'Config'    },
            { icon: 'i-solar-key-bold-duotone',             label: 'API Keys'  },
            { icon: 'i-solar-clipboard-list-bold-duotone',  label: 'Audit Log' },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-lg {item.icon}"></span>
              {item.label}
            </button>
          {/each}
        </div>
      {/if}

    </nav>

    <!-- Bottom: theme + settings + sign out -->
    <div class="border-t border-surface-z0 px-3 py-3 space-y-1">
      <div class="flex items-center justify-between px-2.5 py-2">
        <span class="text-xs text-surface-z4">Theme</span>
        <ThemeSwitcherToggle />
      </div>
      <a
        href="/settings"
        class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7"
      >
        <span class="text-lg i-solar-settings-bold-duotone"></span>
        Settings
      </a>
      <button
        class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7"
        onclick={() => kavach.signOut()}
      >
        <span class="text-lg i-solar-logout-3-bold-duotone"></span>
        {context === 'platform' ? 'Switch tenant' : 'Sign out'}
      </button>
    </div>
  </aside>

  <!-- ══ MAIN COLUMN ════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col overflow-hidden">

    <!-- Mobile header -->
    <header class="flex h-12 shrink-0 items-center justify-between border-b border-surface-z0 bg-surface-z2 px-4 md:hidden">
      <button
        class="flex h-8 w-8 items-center justify-center rounded-lg text-surface-z5 hover:bg-surface-z3 hover:text-surface-z8"
        onclick={() => sidebarOpen = true}
        aria-label="Open menu"
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" class="h-4 w-4">
          <path stroke-linecap="round" d="M2 4h12M2 8h12M2 12h12"/>
        </svg>
      </button>
      <div class="flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-md bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="font-semibold text-surface-z8 text-sm">sensei</span>
      </div>
      <div class="w-8"></div>
    </header>

    <!-- Content -->
    <main class="flex-1 overflow-y-auto">
      {@render children()}
    </main>
  </div>
</div>
