<script lang="ts">
    import { onMount } from 'svelte';
    import { memoryState } from '$lib/memoryState.svelte.js';
    import { appState } from '$lib/appstate.svelte.js';
    import TriageList  from './TriageList.svelte';
    import ActiveList  from './ActiveList.svelte';
    import ArchiveList from './ArchiveList.svelte';
    import MemoryDetail from './MemoryDetail.svelte';

    type Tab = 'triage' | 'active' | 'archive';
    let tab = $state<Tab>('triage');

    onMount(async () => {
        await memoryState.load(appState.activeProjectId ?? undefined);
        if (memoryState.triageCount === 0) tab = 'active';
    });
</script>

<div class="learnings-page" data-testid="learnings-page">
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

    <div class="layout">
        <section class="list" data-testid={`list-${tab}`}>
            {#if tab === 'triage'}<TriageList />
            {:else if tab === 'active'}<ActiveList />
            {:else}<ArchiveList />{/if}
        </section>

        <aside class="detail" data-testid="detail-pane">
            <MemoryDetail />
        </aside>
    </div>
</div>

<style>
    .learnings-page { display: flex; flex-direction: column; height: 100%; }
    .tabs { display: flex; gap: 0.5rem; border-bottom: 1px solid var(--surface-z3); padding: 0.5rem 1rem; }
    .tab { padding: 0.5rem 1rem; border: none; background: transparent; cursor: pointer; }
    .tab.active { border-bottom: 2px solid var(--accent); font-weight: 600; }
    .count { margin-left: 0.5rem; opacity: 0.6; }
    .layout { display: grid; grid-template-columns: 1fr 1fr; flex: 1; min-height: 0; }
    .list { overflow-y: auto; padding: 1rem; border-right: 1px solid var(--surface-z3); }
    .detail { overflow-y: auto; padding: 1rem; }
</style>
