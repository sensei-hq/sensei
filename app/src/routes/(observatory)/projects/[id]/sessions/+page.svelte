<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ProjectSession } from '$lib/types.js';

  const projectId = $derived(page.params.id ?? '');
  let sessions = $state<ProjectSession[]>([]);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const resp = await api.getProjectSessions(projectId);
    sessions = resp.sessions ?? [];
    loading = false;
  });

  const fmtDate = (iso: string) => new Date(iso).toLocaleDateString();
</script>

<div class="max-w-[960px] mx-auto px-12 py-10 pb-16">
  <h1 class="display text-xl font-normal m-0 mb-6">Sessions</h1>
  {#if loading}
    <p class="text-sm text-ink-soft">Loading sessions…</p>
  {:else if sessions.length === 0}
    <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
      <span class="kanji text-3xl text-accent opacity-40">録</span>
      <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
        No sessions recorded for this project yet.
      </p>
    </div>
  {:else}
    <div class="grid grid-cols-[1fr_100px_80px_80px_100px] gap-3 px-3 py-2 text-xs text-ink-soft tracking-wide uppercase">
      <span>Task</span>
      <span class="text-right">Turns</span>
      <span class="text-right">FTR</span>
      <span class="text-right">Outcome</span>
      <span class="text-right">Started</span>
    </div>
    {#each sessions as session (session.id)}
      <div class="grid grid-cols-[1fr_100px_80px_80px_100px] gap-3 px-3 py-2.5 border-b border-paper-mute text-sm items-center">
        <span class="text-ink truncate">{session.task}</span>
        <span class="text-right font-mono text-xs text-ink-soft">{session.turns}</span>
        <span class="text-right font-mono text-xs" class:text-success={session.ftr}>{session.ftr ? '✓' : '✗'}</span>
        <span class="text-right text-xs text-ink-soft">{session.outcome ?? '—'}</span>
        <span class="text-right text-xs text-ink-soft">{fmtDate(session.started_at)}</span>
      </div>
    {/each}
  {/if}
</div>
