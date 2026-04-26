<script lang="ts">
  import { goto } from '$app/navigation';
  import { senseiApi } from '$lib/api.js';
  import { getPort, loadAppState, isSetupComplete } from '$lib/appstate.svelte.js';

  let checking = $state(false);
  let error = $state<string | null>('Daemon not reachable');

  async function retry() {
    checking = true;
    error = null;

    try {
      await loadAppState();
      const api = senseiApi(getPort());
      const health = await api.getHealth();

      if (!health?.ok) {
        error = 'Daemon reports unhealthy status';
        return;
      }

      // Health passed — route through gate logic
      if (isSetupComplete()) {
        goto('/observatory', { replaceState: true });
      } else {
        goto('/config', { replaceState: true });
      }
    } catch {
      error = 'Daemon not reachable';
    } finally {
      checking = false;
    }
  }
</script>

<div class="health-content">
  <span class="kanji hero-kanji">支</span>

  <h1 class="display hero-title">
    {#if checking}
      Checking…
    {:else}
      Waiting for the daemon.
    {/if}
  </h1>

  <p class="hero-body">
    {#if error}
      {error}. Make sure <code>senseid</code> is running on port {getPort()}.
    {:else}
      Verifying prerequisites…
    {/if}
  </p>

  <button class="btn-solid" onclick={retry} disabled={checking}>
    {checking ? 'Checking…' : 'Retry'}
  </button>

  <p class="hint">
    Start the daemon with <code>senseid serve</code> or <code>brew services start sensei</code>
  </p>
</div>

<style>
  .health-content {
    text-align: center;
    max-width: 480px;
  }
  .hero-kanji {
    font-size: 64px;
    color: var(--shu);
    opacity: 0.4;
    display: block;
    margin-bottom: 20px;
  }
  .hero-title {
    font-size: 28px;
    font-weight: 300;
    margin: 0 0 12px;
    letter-spacing: -0.01em;
  }
  .hero-body {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.6;
    margin: 0 0 28px;
  }
  .hero-body code {
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 1px 5px;
    background: var(--paper-2);
    border-radius: 3px;
  }
  .hint {
    margin-top: 20px;
    font-size: 11px;
    color: var(--sumi-4);
  }
  .hint code {
    font-family: var(--font-mono);
    font-size: 11px;
  }
</style>
