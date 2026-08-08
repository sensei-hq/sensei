<script lang="ts">
    import { invalidateAll } from '$app/navigation';
    import { PageHeader, ScreenState } from '$lib/components';
    import { Toggle } from '@rokkit/ui';
    import type { ProxyItem } from '@rokkit/states';
    let { data } = $props();

    // Local search + filter — libraries can grow to hundreds on a monorepo.
    let query = $state('');
    let filter = $state<'all' | 'wrapped' | 'local' | 'conflict'>('all');

    // rokkit Toggle options. `disabled` disables the Conflicts pill when the
    // project has no version conflicts (mirrors the old hand-rolled guard). The
    // per-option `data-testid` is emitted in the itemContent snippet since the
    // Toggle owns its option button; the click delegates up to it.
    const filterOptions = $derived([
        { value: 'all', label: 'All' },
        { value: 'wrapped', label: 'Wrapped' },
        { value: 'local', label: 'Local' },
        { value: 'conflict', label: 'Conflicts', disabled: data.conflicts.length === 0 },
    ]);

    // Set of library names known to have a version conflict, for badge lookup.
    const conflictSet = $derived(new Set(data.conflicts.map(c => c.library_name)));

    const filtered = $derived(
        data.libraries.filter(l => {
            const q = query.trim().toLowerCase();
            if (q && !l.name.toLowerCase().includes(q) && !(l.ecosystem?.toLowerCase().includes(q))) return false;
            switch (filter) {
                case 'wrapped':  return l.hasDocs;
                case 'local':    return !!l.localSource;
                case 'conflict': return conflictSet.has(l.name);
                default:         return true;
            }
        }),
    );
</script>

<PageHeader title="Libraries">
    {#snippet right()}
        <div class="text-sm text-ink-mute flex gap-3 items-center">
            <span>{data.libraries.length} total</span>
            <span class="text-success">{data.wrappedCount} wrapped</span>
            {#if data.localCount > 0}<span class="text-primary">{data.localCount} local</span>{/if}
            {#if data.conflicts.length > 0}
                <span class="text-warning" data-testid="library-conflicts-count">
                    ⚠ {data.conflicts.length} version conflict{data.conflicts.length === 1 ? '' : 's'}
                </span>
            {/if}
        </div>
    {/snippet}
</PageHeader>

<div class="px-6 py-6">
    {#if data.error}
        <ScreenState status="error" error={data.error} onretry={invalidateAll} />
    {:else}
    <!-- Version conflicts banner — the T1a signal. Users decide before browsing rows.
         A paper surface (flips for dark mode) with a warning left-accent + heading — NOT a
         bg-warning-soft fill. warning-soft is a single-pole tint that stays pale (0.95) in dark
         mode, so ink body text (flips to ~0.94) would sit near-white-on-near-white and vanish.
         Paper-soft flips to dark, keeping the ink body text high-contrast in both modes. -->
    {#if data.conflicts.length > 0}
        <section class="mb-6 rounded-md p-3 bg-paper-soft border border-paper-edge border-l-2 border-l-warning" data-testid="library-conflicts-banner">
            <h3 class="text-sm font-medium m-0 mb-2 text-warning flex items-center gap-1.5">
                <span aria-hidden="true">⚠</span> Version conflicts
            </h3>
            <p class="text-xs text-ink-soft m-0 mb-3">
                These libraries are pinned to different versions across folders in this project.
            </p>
            <ul class="flex flex-col gap-2">
                {#each data.conflicts as c (c.library_id)}
                    <li class="text-xs" data-testid={`library-conflict-${c.library_name}`}>
                        <span class="font-mono text-ink font-medium">{c.library_name}</span>
                        <span class="opacity-60"> · {c.ecosystem}</span>
                        <span class="opacity-60"> · </span>
                        <span class="font-mono text-ink">{c.versions.join(', ')}</span>
                        <span class="opacity-60"> across </span>
                        <span class="font-mono text-ink-soft">{c.folders.join(', ')}</span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    <div class="flex gap-2 items-center mb-4 flex-wrap">
        <input
            type="text"
            placeholder="Search by name or ecosystem…"
            bind:value={query}
            class="px-3 py-1.5 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none flex-1 min-w-[200px]"
            data-testid="library-search"
        />
        <Toggle
            options={filterOptions}
            value={filter}
            onchange={(v: unknown) => (filter = v as typeof filter)}
            label="Library filter"
        >
            {#snippet itemContent(proxy: ProxyItem)}
                <span data-testid={`library-filter-${String(proxy.value)}`}>{proxy.label}</span>
            {/snippet}
        </Toggle>
    </div>

    {#if filtered.length === 0}
        <p class="text-sm text-ink-mute">
            {data.libraries.length === 0
                ? 'No libraries associated with this project yet.'
                : 'No libraries match this filter.'}
        </p>
    {:else}
        <ul class="list-none m-0 p-0" data-testid="library-list">
            {#each filtered as lib (lib.id)}
                {@const inConflict = conflictSet.has(lib.name)}
                <li class="lib-row flex items-center gap-2.5 py-2 border-b border-paper-edge text-sm"
                    data-testid={`library-row-${lib.name}`}>
                    <span class="font-semibold text-ink truncate max-w-[280px]">{lib.name}</span>
                    <span class="text-ink-mute text-xs">{lib.ecosystem}</span>
                    <div class="flex gap-1 flex-1">
                        {#if lib.hasDocs}
                            <span class="badge bg-success-soft text-success border border-success-edge"
                                  title="{lib.pageCount ?? 0} indexed doc page{lib.pageCount === 1 ? '' : 's'}"
                                  data-testid={`library-badge-wrapped-${lib.name}`}>Wrapped</span>
                        {/if}
                        {#if lib.localSource}
                            <span class="badge bg-primary-soft text-primary border border-primary-edge"
                                  title={lib.localSource}
                                  data-testid={`library-badge-local-${lib.name}`}>Local</span>
                        {/if}
                        {#if inConflict}
                            <span class="badge bg-warning-soft text-warning border border-warning-edge"
                                  data-testid={`library-badge-conflict-${lib.name}`}>Version conflict</span>
                        {/if}
                    </div>
                    <span class="scope-badge text-xs px-1.5 py-px rounded-md font-mono"
                          class:global={lib.scope === 'global'}
                          class:proj={lib.scope === 'project'}>
                        [{lib.scope}]
                    </span>
                </li>
            {/each}
        </ul>
    {/if}
    {/if}
</div>

<style>
    .lib-row:last-child {
        border-bottom: none;
    }
    .lib-row:hover {
        background: var(--paper-mute);
    }
    .scope-badge.global {
        background: var(--paper-mute);
        opacity: 0.7;
    }
    .scope-badge.proj {
        background: var(--accent-soft);
        color: var(--accent);
    }
    .badge {
        padding: 1px 8px;
        border-radius: 999px;
        font-size: 10px;
        line-height: 1.4;
        white-space: nowrap;
    }
</style>
