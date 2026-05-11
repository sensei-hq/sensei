<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<svelte:head>
  <title>{title} — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="/" class="back-link">
      <span class="kanji" style="font-size: 18px; color: var(--shu);">先生</span>
      <span class="display" style="font-size: 15px;">Sensei</span>
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
    padding: 24px 56px;
    border-bottom: var(--hairline);
  }
  .back-link {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    text-decoration: none;
  }
  .page-content {
    max-width: 800px;
    margin: 0 auto;
    padding: 80px 56px 120px;
  }
  @media (max-width: 900px) {
    .page-nav { padding: 20px 24px; }
    .page-content { padding: 48px 24px 80px; }
  }
</style>
