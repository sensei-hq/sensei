<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById } from '$lib/solutions.js';
  import { senseiApi } from '$lib/api.js';

  let solution = $derived(getSolutionById($page.params.id));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));

  let allSessions = $state<Array<{
    id: string; task: string; project: string; startedAt: string;
    completedAt?: string; outcome?: string; ftr?: number | null;
    cost?: number; tokensIn?: number; tokensOut?: number;
  }>>([]);

  let repoIds = $derived(new Set(solution?.repos.map(r => r.repoId) ?? []));
  let sessions = $derived(allSessions.filter(s => repoIds.has(s.project)));

  let expandedId = $state<string | null>(null);

  function ftrClass(ftr: number | null | undefined): string {
    if (ftr == null) return 'text-surface-z4';
    if (ftr >= 0.8) return 'text-success-z6';
    if (ftr >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  function formatDate(iso: string): string {
    try { return new Date(iso).toLocaleDateString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); }
    catch { return iso; }
  }

  async function load() {
    const data = await senseiApi(port).getSessions();
    allSessions = data.sessions;
  }

  onMount(() => { load(); });
</script>

<div class="flex-1 overflow-y-auto px-5 py-4 space-y-3">
  <p class="text-xs text-surface-z4">{sessions.length} session{sessions.length === 1 ? '' : 's'} across {repoIds.size} repo{repoIds.size === 1 ? '' : 's'}</p>

  {#if sessions.length === 0}
    <div class="text-center py-12">
      <p class="text-sm text-surface-z4">No sessions yet for this solution.</p>
      <p class="text-xs text-surface-z3 mt-1">Sessions appear after you use sensei MCP tools in your ACP.</p>
    </div>
  {:else}
    <div class="space-y-1">
      {#each sessions as s (s.id)}
        {@const expanded = expandedId === s.id}
        <div class="rounded-lg bg-surface-z2/50 overflow-hidden">
          <button
            onclick={() => expandedId = expanded ? null : s.id}
            class="flex w-full items-center gap-3 px-3 py-2.5 text-left hover:bg-surface-z2 transition-colors"
          >
            <span class="flex-1 text-sm text-surface-z7 truncate">{s.task}</span>
            <span class="text-[10px] text-surface-z4 shrink-0">{formatDate(s.startedAt)}</span>
            <span class="w-10 text-right text-xs font-mono {ftrClass(s.ftr)} shrink-0">
              {s.ftr != null ? `${Math.round(s.ftr * 100)}%` : '—'}
            </span>
          </button>
          {#if expanded}
            <div class="border-t border-surface-z0/20 px-3 py-2 flex gap-4 text-xs text-surface-z4">
              <span>{s.outcome ?? 'in progress'}</span>
              <span>${(s.cost ?? 0).toFixed(2)}</span>
              <span>{Math.round((s.tokensIn ?? 0) / 1000)}k in / {Math.round((s.tokensOut ?? 0) / 1000)}k out</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
