<script lang="ts">
  import type { Snippet } from 'svelte';
  import { base } from '$app/paths';

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<svelte:head>
  <title>{title} — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="{base}/sensei" class="back-link">
      <span class="i-brand:sensei text-sensei" style="width:20px;height:20px;flex-shrink:0" aria-hidden="true"></span>
      <span class="display text-base">Sensei</span>
    </a>
  </nav>

  <div class="page-content">
    {@render children()}
  </div>
</div>

<style>
  .page {
    background: var(--paper);
    color: var(--sumi);
    min-height: 100vh;
    font-family: var(--font-ui);
  }
  .page-nav {
    padding: 24px 48px;
    border-bottom: var(--hairline);
  }
  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    text-decoration: none;
  }
  .page-content {
    max-width: 800px;
    margin: 0 auto;
    padding: 64px 48px 120px;
  }
  @media (max-width: 900px) {
    .page-nav { padding: 16px 24px; }
    .page-content { padding: 48px 24px 64px; }
  }
</style>
