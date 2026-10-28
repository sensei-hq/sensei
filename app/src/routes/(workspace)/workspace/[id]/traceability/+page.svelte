<script lang="ts">
    import { PageHeader, Screen, ScreenState, StatusDot } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { DriftScan, driftDotStatus } from '$lib/traceability.svelte.js';
    import { invalidateAll } from '$app/navigation';
    import { page } from '$app/state';

    let { data } = $props();

    // The dot's tone is the shared drift rule (an unknown status is neutral, as on
    // the observatory screen); the scan action and its flags are DriftScan's.
    const workspaceId = $derived(page.params.id ?? '');
    const drift = new DriftScan((id) => senseiApi(appState.port).tryScanWorkspaceDrift(id), () => invalidateAll());

    // Expandable-row state — only one row shows its expected-vs-actual diff
    // at a time so the list stays readable.
    let openedItemId = $state<string | null>(null);
</script>

<Screen>
    {#snippet header()}
    <PageHeader title="Traceability">
        {#snippet right()}
            <div class="text-sm text-ink-soft flex items-center gap-3">
                {#if !data.error}
                    <span>{data.total} tracked</span>
                    <span> · </span>
                    <span class="text-warning">{data.drifted} drifted</span>
                    <span> · </span>
                    <span class="text-danger">{data.broken} broken</span>
                {/if}
                <button
                    type="button"
                    class="px-3 py-1 rounded-md text-xs border border-paper-edge bg-transparent text-ink cursor-pointer"
                    disabled={drift.scanning}
                    data-testid="drift-scan-button"
                    onclick={() => drift.run(workspaceId)}
                >{drift.buttonLabel}</button>
            </div>
        {/snippet}
    </PageHeader>
    {/snippet}
    <div class="px-6 py-6">
        {#if drift.errorText}
            <div class="mb-4 text-xs text-danger" data-testid="drift-scan-error">{drift.errorText}</div>
        {/if}
        {#if data.error}
            <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
        {:else}
        {#if drift.summaryText}
            <div class="mb-4 text-xs text-ink-mute" data-testid="drift-scan-summary">{drift.summaryText}</div>
        {/if}
        {#if data.driftItems.length === 0 && data.indexedDocs === 0}
            <!-- Nothing to check (#290): no document is indexed, so an empty list is
                 not "no drift". Markdown has no indexer adapter yet. -->
            <p class="text-sm text-ink-mute" data-testid="drift-no-docs">
                No documents are indexed for this workspace, so nothing has been checked for drift — sensei does not
                index markdown yet.
            </p>
        {:else if data.driftItems.length === 0}
            <p class="text-sm text-ink-mute">No drift signals yet. Run a scan to check for stale doc references.</p>
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
                            <span class="mt-1"><StatusDot status={driftDotStatus(item.status)} /></span>
                            <span class="flex-1">{item.detail ?? item.status}</span>
                            {#if hasDiff}
                                <span class="text-xs text-ink-soft">{opened ? '−' : '+'}</span>
                            {/if}
                        </button>
                        {#if opened && hasDiff}
                            <div class="pl-6 pb-3 flex flex-col gap-2" data-testid={`drift-diff-${item.id}`}>
                                {#if item.expectedSignature}
                                    <div>
                                        <p class="text-xs text-ink-soft uppercase tracking-wide m-0 mb-1">Expected</p>
                                        <pre class="text-xs font-mono bg-paper-soft border border-paper-edge rounded-md p-2 m-0 whitespace-pre-wrap break-all">{item.expectedSignature}</pre>
                                    </div>
                                {/if}
                                {#if item.actualSignature}
                                    <div>
                                        <p class="text-xs text-ink-soft uppercase tracking-wide m-0 mb-1">Actual</p>
                                        <pre class="text-xs font-mono bg-paper-soft border border-paper-edge rounded-md p-2 m-0 whitespace-pre-wrap break-all">{item.actualSignature}</pre>
                                    </div>
                                {:else if item.status === 'broken'}
                                    <div>
                                        <p class="text-xs text-ink-soft uppercase tracking-wide m-0 mb-1">Actual</p>
                                        <p class="text-xs text-warning italic m-0">Symbol no longer exists.</p>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </li>
                {/each}
            </ul>
        {/if}
        {/if}
    </div>
</Screen>

<style>
    .drift-row:last-child {
        border-bottom: none;
    }
</style>
