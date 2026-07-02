<script lang="ts">
    import { PageHeader, StatusDot } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { invalidateAll } from '$app/navigation';
    import { page } from '$app/state';

    let { data } = $props();

    type DriftStatus = 'drifted' | 'broken' | string;
    const dotStatus = (s: DriftStatus): 'ok' | 'warn' | 'fail' =>
        s === 'broken' ? 'fail' : s === 'drifted' ? 'warn' : 'ok';

    // Doc-drift detector state — the button below runs the analyzer
    // on-demand until the scheduled job lands (T3 Slice 2.3 full).
    const projectId = $derived(page.params.id ?? '');
    let scanning = $state(false);
    let lastSummary = $state<{ scannedDocs: number; newBroken: number; resolved: number } | null>(null);

    async function scan() {
        if (!projectId) return;
        scanning = true;
        lastSummary = null;
        try {
            const res = await fetch(`http://127.0.0.1:${appState.port}/api/projects/${encodeURIComponent(projectId)}/drift/scan`, { method: 'POST' });
            if (res.ok) {
                lastSummary = await res.json();
                await invalidateAll();
            }
        } finally {
            scanning = false;
        }
        // Keep senseiApi import referenced so tree-shaking doesn't drop it
        // if this file ever re-adds a wrapper.
        void senseiApi;
    }
</script>

<PageHeader title="Traceability">
    {#snippet right()}
        <div class="text-sm text-ink-mute flex items-center gap-3">
            <span>{data.total} tracked</span>
            <span> · </span>
            <span class="text-warning">{data.drifted} drifted</span>
            <span> · </span>
            <span class="text-danger">{data.broken} broken</span>
            <button
                type="button"
                class="px-3 py-1 rounded-md text-xs border border-paper-edge bg-transparent text-ink cursor-pointer"
                disabled={scanning}
                onclick={scan}
            >{scanning ? 'Scanning…' : 'Scan now'}</button>
        </div>
    {/snippet}
</PageHeader>
<div class="px-6 py-6">
    {#if lastSummary}
        <div class="mb-4 text-xs text-ink-soft">
            Scanned {lastSummary.scannedDocs} docs · added {lastSummary.newBroken} broken · resolved {lastSummary.resolved}.
        </div>
    {/if}
    {#if data.driftItems.length === 0}
        <p class="text-sm text-ink-soft">No drift signals yet. Run a scan to check for stale doc references.</p>
    {:else}
        <ul class="list-none m-0 p-0">
            {#each data.driftItems as item (item.id)}
                <li class="drift-row flex gap-2.5 py-2 border-b border-paper-mute text-sm items-start">
                    <span class="mt-1"><StatusDot status={dotStatus(item.status)} /></span>
                    <span class="flex-1">{item.detail ?? item.status}</span>
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .drift-row:last-child {
        border-bottom: none;
    }
</style>
