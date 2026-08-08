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

    // Expandable-row state — only one row shows its expected-vs-actual diff
    // at a time so the list stays readable.
    let openedItemId = $state<string | null>(null);
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
                data-testid="drift-scan-button"
                onclick={scan}
            >{scanning ? 'Scanning…' : 'Scan now'}</button>
        </div>
    {/snippet}
</PageHeader>
<div class="px-6 py-6">
    {#if lastSummary}
        <div class="mb-4 text-xs text-ink-soft" data-testid="drift-scan-summary">
            Scanned {lastSummary.scannedDocs} docs · added {lastSummary.newBroken} broken · resolved {lastSummary.resolved}.
        </div>
    {/if}
    {#if data.driftItems.length === 0}
        <p class="text-sm text-ink-soft">No drift signals yet. Run a scan to check for stale doc references.</p>
    {:else}
        <ul class="list-none m-0 p-0" data-testid="drift-list">
            {#each data.driftItems as item (item.id)}
                {@const opened = openedItemId === item.id}
                {@const hasDiff = !!(item.expectedSignature || item.actualSignature)}
                <li class="drift-row border-b border-paper-edge text-sm" data-testid="drift-row" data-opened={opened || undefined}>
                    <button
                        type="button"
                        class="w-full flex gap-2.5 py-2 items-start bg-transparent border-none cursor-pointer text-left text-inherit"
                        data-testid={`drift-toggle-${item.id}`}
                        aria-expanded={opened}
                        onclick={() => (openedItemId = opened ? null : item.id)}
                    >
                        <span class="mt-1"><StatusDot status={dotStatus(item.status)} /></span>
                        <span class="flex-1">{item.detail ?? item.status}</span>
                        {#if hasDiff}
                            <span class="text-xs text-ink-mute">{opened ? '−' : '+'}</span>
                        {/if}
                    </button>
                    {#if opened && hasDiff}
                        <div class="pl-6 pb-3 flex flex-col gap-2" data-testid={`drift-diff-${item.id}`}>
                            {#if item.expectedSignature}
                                <div>
                                    <p class="text-xs text-ink-mute uppercase tracking-wide m-0 mb-1">Expected</p>
                                    <pre class="text-xs font-mono bg-paper-soft border border-paper-edge rounded-md p-2 m-0 whitespace-pre-wrap break-all">{item.expectedSignature}</pre>
                                </div>
                            {/if}
                            {#if item.actualSignature}
                                <div>
                                    <p class="text-xs text-ink-mute uppercase tracking-wide m-0 mb-1">Actual</p>
                                    <pre class="text-xs font-mono bg-paper-soft border border-paper-edge rounded-md p-2 m-0 whitespace-pre-wrap break-all">{item.actualSignature}</pre>
                                </div>
                            {:else if item.status === 'broken'}
                                <div>
                                    <p class="text-xs text-ink-mute uppercase tracking-wide m-0 mb-1">Actual</p>
                                    <p class="text-xs text-warning italic m-0">Symbol no longer exists.</p>
                                </div>
                            {/if}
                        </div>
                    {/if}
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
