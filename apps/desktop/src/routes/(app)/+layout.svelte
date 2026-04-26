<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { loadAppState, getPort } from '$lib/appstate.svelte.js';

  let { children } = $props();

  const NAV_ITEMS = [
    { href: '/observatory', kanji: '家', label: 'Today' },
  ];

  onMount(async () => {
    await loadAppState();
  });
</script>

<div class="app-shell">
  <div class="drag-spacer drag-region"></div>

  <div class="app-body">
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="kanji" style="font-size: 20px; color: var(--shu);">先</span>
        <span class="display" style="font-size: 16px;">Sensei</span>
      </div>

      <div class="sidebar-section">
        <p class="sidebar-label">Observatory</p>
        <nav class="sidebar-nav">
          {#each NAV_ITEMS as item (item.href)}
            {@const active = $page.url.pathname === item.href}
            <a
              href={item.href}
              class="nav-item"
              class:active
            >
              <span class="kanji nav-kanji" class:active>{item.kanji}</span>
              <span>{item.label}</span>
            </a>
          {/each}
        </nav>
      </div>

      <div class="sidebar-footer">
        <span class="mono daemon-status">daemon · port {getPort()}</span>
      </div>
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

  /* ── Main ─────────────────────────────────────────────── */
  .main-content {
    overflow: auto;
  }
</style>
