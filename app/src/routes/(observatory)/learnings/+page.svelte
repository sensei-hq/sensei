<script lang="ts">
    // On the ScreenShell (D87 §9): the header's mark/title/eyebrow are the
    // shell's; the tabs stay atop the body, the list is the body, the detail
    // pane is the shell's sidebar — each scrolled by the shell, not the page.
    import { StateView } from '@sensei/ui';
    import { onMount } from 'svelte';
    import { memoryState } from '$lib/memoryState.svelte.js';
    import { appState } from '$lib/appstate.svelte.js';
    import TriageList  from './TriageList.svelte';
    import ActiveList  from './ActiveList.svelte';
    import ArchiveList from './ArchiveList.svelte';
    import MemoryDetail from './MemoryDetail.svelte';
    import { ShellScreen } from '@sensei/ui';
    import { railIcon } from '../observatory-nav';

    type Tab = 'triage' | 'active' | 'archive';
    let tab = $state<Tab>('triage');

    onMount(async () => {
        await memoryState.load(appState.activeWorkspaceId ?? undefined);
        if (memoryState.triageCount === 0) tab = 'active';
    });
</script>

<ShellScreen
    viewId="observatory.learnings"
    mark={{ kanji: '覚', icon: railIcon('/learnings') }}
    title="Memories"
    context="Observatory"
    search={false}
    data-testid="learnings-page"
>
    {#snippet sidebar()}
        <aside class="detail" data-testid="detail-pane">
            <MemoryDetail />
        </aside>
    {/snippet}
    <div>
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
        <section class="list" data-testid={`list-${tab}`}>
            {#if memoryState.listError}
                <!-- The lists could not be read or refreshed (#288) — not an empty tab.
                     After a failed refresh the last lists still show below it. -->
                <StateView
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
    </div>
</ShellScreen>

<style>
    .tabs { display: flex; gap: 0.5rem; border-bottom: 1px solid var(--paper-mute); padding: 0.5rem 1rem; }
    .tab { padding: 0.5rem 1rem; border: none; background: transparent; cursor: pointer; }
    .tab.active { border-bottom: 2px solid var(--accent); font-weight: 600; }
    .count { margin-left: 0.5rem; opacity: 0.6; }
    .list { padding: 1rem; }
    .detail { padding: 1rem; }
</style>
