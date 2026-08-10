<script lang="ts">
  import type { Snippet } from 'svelte';
  import { base } from '$app/paths';

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<svelte:head>
  <title>{title} — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav py-6 px-12">
    <a href="{base}/sensei" class="back-link gap-2">
      <span class="i-brand:sensei text-sensei" style="width:20px;height:20px;flex-shrink:0" aria-hidden="true"></span>
      <span class="display text-base">Sensei</span>
    </a>
  </nav>

  <div class="page-content pt-16 px-12 pb-30">
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
    border-bottom: var(--hairline);
  }
  .back-link {
    display: inline-flex;
    align-items: center;
    text-decoration: none;
  }
  .page-content {
    max-width: 800px;
    margin: 0 auto;
  }
  @media (max-width: 900px) {
    .page-nav { padding: 16px 24px; }
    .page-content { padding: 48px 24px 64px; }
  }
</style>
