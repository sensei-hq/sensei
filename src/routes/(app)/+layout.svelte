<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  let { children } = $props();

  const NAV_ITEMS = [
    { href: '/observatory', kanji: '家', label: 'Today' },
    { href: '/sessions',    kanji: '刻', label: 'Sessions' },
    { href: '/learnings',   kanji: '學', label: 'Learnings' },
    { href: '/libraries',   kanji: '書', label: 'Libraries' },
    { href: '/instruments',  kanji: '具', label: 'Instruments' },
  ];

  const BOTTOM_ITEMS = [
    { href: '/logs',     kanji: '録', label: 'Logs' },
    { href: '/settings', kanji: '設', label: 'Settings' },
  ];

  type SidebarProject = { id: string; name: string; kanji: string };

  let projects = $state<SidebarProject[]>([]);
  let sidebarCollapsed = $state(false);

  onMount(async () => {
    await appState.load();
    const api = senseiApi(appState.port);
    const raw = await api.listProjects();
    projects = raw.map((p: any) => ({
      id: p.id,
      name: p.name,
      kanji: p.icon?.value ?? '場',
    }));
  });

  function isActive(href: string): boolean {
    return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
  }
</script>

<div class="app-shell">
  <div class="drag-spacer drag-region"></div>

  <div class="app-body" class:collapsed={sidebarCollapsed}>
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="kanji" style="font-size: 20px; color: var(--shu);">先</span>
        {#if !sidebarCollapsed}
          <span class="display" style="font-size: 16px;">Sensei</span>
          <button class="collapse-btn" onclick={() => sidebarCollapsed = true}>‹</button>
        {/if}
      </div>

      {#if sidebarCollapsed}
        <nav class="sidebar-nav">
          {#each NAV_ITEMS as item (item.href)}
            {@const active = isActive(item.href)}
            <a href={item.href} class="nav-item icon-only" class:active title={item.label}>
              <span class="kanji nav-kanji" class:active>{item.kanji}</span>
            </a>
          {/each}
        </nav>

        {#if projects.length > 0}
          <div class="sidebar-divider"></div>
          <nav class="sidebar-nav">
            {#each projects as proj (proj.id)}
              {@const active = isActive(`/projects/${proj.id}`)}
              <a href="/projects/{proj.id}" class="nav-item icon-only" class:active title={proj.name}>
                <span class="kanji nav-kanji" class:active>{proj.kanji}</span>
              </a>
            {/each}
          </nav>
        {/if}

        <div class="sidebar-footer">
          <button class="collapse-btn" onclick={() => sidebarCollapsed = false}>›</button>
        </div>
      {:else}
        <div class="sidebar-section">
          <p class="sidebar-label">Observatory</p>
          <nav class="sidebar-nav">
            {#each NAV_ITEMS as item (item.href)}
              {@const active = isActive(item.href)}
              <a href={item.href} class="nav-item" class:active>
                <span class="kanji nav-kanji" class:active>{item.kanji}</span>
                <span>{item.label}</span>
              </a>
            {/each}
          </nav>
        </div>

        {#if projects.length > 0}
          <div class="sidebar-section">
            <p class="sidebar-label">Projects</p>
            <nav class="sidebar-nav">
              {#each projects as proj (proj.id)}
                {@const active = isActive(`/projects/${proj.id}`)}
                <a href="/projects/{proj.id}" class="nav-item" class:active>
                  <span class="kanji nav-kanji" class:active>{proj.kanji}</span>
                  <span>{proj.name}</span>
                </a>
              {/each}
            </nav>
          </div>
        {/if}

        <div class="sidebar-section" style="margin-top: auto;">
          <nav class="sidebar-nav">
            {#each BOTTOM_ITEMS as item (item.href)}
              {@const active = isActive(item.href)}
              <a href={item.href} class="nav-item" class:active>
                <span class="kanji nav-kanji" class:active>{item.kanji}</span>
                <span>{item.label}</span>
              </a>
            {/each}
          </nav>
        </div>

        <div class="sidebar-footer">
          <span class="mono daemon-status">daemon · port {appState.port}</span>
        </div>
      {/if}
    </aside>

    <!-- Main content -->
    <main class="main-content">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .app-shell {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
    overflow: hidden;
  }
  .drag-spacer {
    height: 32px;
    flex-shrink: 0;
  }
  .app-body {
    flex: 1;
    display: grid;
    grid-template-columns: 220px 1fr;
    min-height: 0;
    transition: grid-template-columns 0.15s ease;
  }
  .app-body.collapsed {
    grid-template-columns: 52px 1fr;
  }

  /* ── Sidebar ──────────────────────────────────────────── */
  .sidebar {
    border-right: var(--hairline);
    padding: 22px 14px;
    background: var(--paper-2);
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow: auto;
  }
  .sidebar-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 0 6px;
  }
  .sidebar-section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sidebar-label {
    font-size: 9.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    padding: 0 10px 8px;
    margin: 0;
  }
  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: var(--radius);
    font-size: 13px;
    color: var(--sumi-2);
    text-decoration: none;
    transition: background 0.12s;
  }
  .nav-item:hover {
    background: var(--paper-3);
  }
  .nav-item.active {
    background: var(--paper-3);
    color: var(--sumi);
  }
  .nav-kanji {
    font-size: 13px;
    width: 14px;
    color: var(--sumi-3);
  }
  .nav-kanji.active {
    color: var(--shu);
  }
  .sidebar-footer {
    margin-top: auto;
    padding: 10px 10px 0;
    border-top: var(--hairline);
  }
  .daemon-status {
    font-size: 10px;
    color: var(--sumi-3);
  }

  .sidebar-divider {
    height: 1px;
    background: var(--paper-edge);
    margin: 4px 10px;
  }
  .nav-item.icon-only {
    justify-content: center;
    padding: 7px 0;
  }
  .collapse-btn {
    margin-left: auto;
    background: none;
    border: none;
    color: var(--sumi-3);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: var(--radius);
  }
  .collapse-btn:hover {
    background: var(--paper-3);
    color: var(--sumi-2);
  }

  /* ── Main ─────────────────────────────────────────────── */
  .main-content {
    overflow: auto;
  }
</style>
