<script lang="ts">
    import { PageHeader } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { page } from '$app/state';

    let { data } = $props();
    let p = $derived(data.project);
    const projectId = $derived(page.params.id ?? '');

    // Editable copy of the identity-level fields the daemon persists in the
    // project settings JSONB. Read-only fields (id, timestamps, stack derived
    // from indexing) stay off this form.
    let editing = $state(false);
    let saving = $state(false);
    let saveError = $state<string | null>(null);
    let form = $state({
        name: '',
        client: '',
        goal: '',
        preferred_acp: '',
        maturity: '',
    });

    // Reset the form whenever a fresh project loads.
    $effect(() => {
        if (p) {
            form = {
                name: p.name ?? '',
                client: p.client ?? '',
                goal: p.goal ?? '',
                preferred_acp: p.preferred_acp ?? '',
                maturity: p.maturity ?? '',
            };
        }
    });

    async function save() {
        if (!projectId) return;
        saving = true;
        saveError = null;
        try {
            // Skip empty strings so the JSONB patch merged by the daemon stays
            // lossless — unset fields don't clobber existing values.
            const patch: Record<string, string> = {};
            for (const [k, v] of Object.entries(form)) {
                if (v && v.length > 0) patch[k] = v;
            }
            await senseiApi(appState.port).updateProject(projectId, patch);
            editing = false;
        } catch (err: unknown) {
            saveError = err instanceof Error ? err.message : String(err);
        } finally {
            saving = false;
        }
    }

    function cancelEdit() {
        if (p) {
            form = {
                name: p.name ?? '',
                client: p.client ?? '',
                goal: p.goal ?? '',
                preferred_acp: p.preferred_acp ?? '',
                maturity: p.maturity ?? '',
            };
        }
        editing = false;
        saveError = null;
    }
</script>

<PageHeader title={p?.name ?? "—"} description={p?.goal ?? undefined} />
<div class="px-6 py-6 max-w-[600px]">
    {#if !editing}
        {#if p?.client}<p class="text-sm text-ink-mute mb-3">Client: {p.client}</p>{/if}

        <section class="mt-5">
            <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
                Repos ({data.repos.length})
            </h3>
            <ul class="list-none m-0 p-0">
                {#each data.repos as repo (repo.id)}
                    <li class="repo-row flex gap-3 py-1.5 text-sm border-b border-paper-mute">
                        <span class="font-semibold">{repo.name}</span>
                        <span class="opacity-50 text-xs font-mono overflow-hidden text-ellipsis">{repo.path}</span>
                    </li>
                {/each}
            </ul>
        </section>

        {#if p?.stack}
            <section class="mt-5">
                <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
                    Stack
                </h3>
                <div class="flex flex-wrap gap-1.5">
                    {#each [...(p.stack?.languages ?? []), ...(p.stack?.frameworks ?? [])] as t}
                        <span class="bg-paper-mute text-xs px-2 py-1 rounded-md">{t}</span>
                    {/each}
                </div>
            </section>
        {/if}

        <div class="mt-6">
            <button
                type="button"
                class="px-3 py-1.5 rounded-md text-xs border border-paper-edge bg-transparent text-ink-soft cursor-pointer"
                onclick={() => (editing = true)}
                disabled={!p}
            >Edit identity</button>
        </div>
    {:else}
        <form
            onsubmit={(e) => {
                e.preventDefault();
                save();
            }}
            class="flex flex-col gap-4"
        >
            {#each [
                { key: 'name', label: 'Name', placeholder: 'Project name' },
                { key: 'client', label: 'Client', placeholder: 'Client / organisation' },
                { key: 'goal', label: 'Goal', placeholder: 'Short goal or purpose' },
                { key: 'preferred_acp', label: 'Preferred assistant', placeholder: 'claude-code | zed | ...' },
                { key: 'maturity', label: 'Maturity', placeholder: 'spike | mvp | mature | archived' },
            ] as field}
                <label class="flex flex-col gap-1">
                    <span class="text-xs uppercase tracking-wide opacity-60">{field.label}</span>
                    <input
                        type="text"
                        placeholder={field.placeholder}
                        class="px-3 py-2 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none"
                        bind:value={form[field.key as keyof typeof form]}
                    />
                </label>
            {/each}

            {#if saveError}
                <p class="text-xs text-danger m-0">Save failed: {saveError}</p>
            {/if}

            <div class="flex gap-2">
                <button
                    type="submit"
                    disabled={saving}
                    class="px-4 py-2 rounded-md text-sm bg-primary text-on-primary border-none cursor-pointer"
                >{saving ? 'Saving…' : 'Save'}</button>
                <button
                    type="button"
                    onclick={cancelEdit}
                    disabled={saving}
                    class="px-4 py-2 rounded-md text-sm bg-transparent text-ink-soft border border-paper-edge cursor-pointer"
                >Cancel</button>
            </div>
        </form>
    {/if}
</div>

<style>
    .repo-row:last-child {
        border-bottom: none;
    }
</style>
