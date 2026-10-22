<script lang="ts">
    import { onMount } from 'svelte';
    import { memoryState } from '$lib/memoryState.svelte.js';
    import { appState } from '$lib/appstate.svelte.js';
    import TriageList  from './TriageList.svelte';
    import ActiveList  from './ActiveList.svelte';
    import ArchiveList from './ArchiveList.svelte';
    import MemoryDetail from './MemoryDetail.svelte';
    import { PageHeader, Screen, ScreenState } from '$lib/components';
    import { railIcon } from '../observatory-nav';

    type Tab = 'triage' | 'active' | 'archive';
    let tab = $state<Tab>('triage');

    onMount(async () => {
        await memoryState.load(appState.activeWorkspaceId ?? undefined);
        if (memoryState.triageCount === 0) tab = 'active';
    });
</script>

<!-- Lists beside the detail pane: each scrolls itself, under a fixed header (#293). -->
<Screen fill data-testid="learnings-page">
    {#snippet header()}
        <PageHeader kanji="覚" icon={railIcon('/learnings')} eyebrow="Observatory" title="Memories" />
        <header class="tabs">
            <button
                type="button"
                class="tab"
                class:active={tab === 'triage'}
                data-testid="tab-triage"
                onclick={() => (tab = 'triage')}
            >
                Triage <span class="count">{memoryState.triageCount}</span>
            </button>
            <button
                type="button"
                class="tab"
                class:active={tab === 'active'}
                data-testid="tab-active"
                onclick={() => (tab = 'active')}
            >
                Active <span class="count">{memoryState.activeCount}</span>
            </button>
            <button
                type="button"
                class="tab"
                class:active={tab === 'archive'}
                data-testid="tab-archive"
                onclick={() => (tab = 'archive')}
            >
                Archive <span class="count">{memoryState.archiveCount}</span>
            </button>
        </header>
    {/snippet}
    <div class="layout">
        <section class="list" data-testid={`list-${tab}`}>
            {#if memoryState.listError}
                <!-- The lists could not be read or refreshed (#288) — not an empty tab.
                     After a failed refresh the last lists still show below it. -->
                <ScreenState
                    status="error"
                    error={memoryState.listError}
                    onretry={() => void memoryState.load(appState.activeWorkspaceId ?? undefined)}
                />
            {/if}
            {#if memoryState.listError && memoryState.triageCount + memoryState.activeCount + memoryState.archiveCount === 0}
                <!-- Nothing read: no empty tab to show beside the error. -->
            {:else if tab === 'triage'}<TriageList />
            {:else if tab === 'active'}<ActiveList />
            {:else}<ArchiveList />{/if}
        </section>

        <aside class="detail" data-testid="detail-pane">
            <MemoryDetail />
        </aside>
    </div>
</Screen>

<style>
    .tabs { display: flex; gap: 0.5rem; border-bottom: 1px solid var(--paper-mute); padding: 0.5rem 1rem; }
    .tab { padding: 0.5rem 1rem; border: none; background: transparent; cursor: pointer; }
    .tab.active { border-bottom: 2px solid var(--accent); font-weight: 600; }
    .count { margin-left: 0.5rem; opacity: 0.6; }
    .layout { display: grid; grid-template-columns: 1fr 1fr; flex: 1; min-height: 0; }
    .list { overflow-y: auto; padding: 1rem; border-right: 1px solid var(--paper-mute); }
    .detail { overflow-y: auto; padding: 1rem; }
</style>
