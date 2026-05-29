<script lang="ts">
    import { PageHeader } from '$lib/components';
    let { data } = $props();
    let p = $derived(data.project);
</script>

<PageHeader title={p?.name ?? "—"} description={p?.goal ?? undefined} />
<div class="px-6 py-6 max-w-[600px]">
    {#if p?.client}<p class="text-sm text-ink-z6 mb-3">Client: {p.client}</p>{/if}

    <section class="mt-5">
        <h3
            class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide"
        >
            Repos ({data.repos.length})
        </h3>
        <ul class="list-none m-0 p-0">
            {#each data.repos as repo (repo.id)}
                <li
                    class="repo-row flex gap-3 py-1.5 text-sm border-b border-surface-z2"
                >
                    <span class="font-semibold">{repo.name}</span>
                    <span
                        class="opacity-50 text-xs font-mono overflow-hidden text-ellipsis"
                        >{repo.path}</span
                    >
                </li>
            {/each}
        </ul>
    </section>

    {#if p?.stack}
        <section class="mt-5">
            <h3
                class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide"
            >
                Stack
            </h3>
            <div class="flex flex-wrap gap-1.5">
                {#each [...(p.stack?.languages ?? []), ...(p.stack?.frameworks ?? [])] as t}
                    <span class="bg-surface-z3 text-xs px-2 py-1 rounded-md"
                        >{t}</span
                    >
                {/each}
            </div>
        </section>
    {/if}
</div>

<style>
    .repo-row:last-child {
        border-bottom: none;
    }
</style>
