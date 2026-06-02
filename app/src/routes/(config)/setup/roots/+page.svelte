<script lang="ts">
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { appState } from "$lib/appstate.svelte.js";

    const roots = $derived(wizardState.roots.roots);
    let adding = $state(false);

    async function addRoot() {
        const trimmed = wizardState.roots.newPath.trim();
        if (!trimmed || roots.some((r) => r.path === trimmed) || adding) return;

        adding = true;
        const api = senseiApi(appState.port);
        const result = await api.addWatchRoot(trimmed);

        if (result.ok) {
            // Use the id and expanded path returned from the daemon
            wizardState.roots.roots = [
                ...roots,
                {
                    id: result.id,
                    path: result.path,
                    name: result.path.split("/").pop() ?? result.path,
                    status: "scanning" as const,
                    excluded: [],
                    repos_found: 0,
                    scanned: false,
                    modified_at: new Date().toISOString(),
                },
            ];
            wizardState.roots.newPath = "";
        }
        adding = false;
    }

    async function removeRoot(id: string) {
        const api = senseiApi(appState.port);
        await api.removeWatchRoot(id);
        wizardState.roots.roots = roots.filter((r) => r.id !== id);
    }
</script>

<div class="max-w-[780px]">
    <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
        Where your work lives. Sensei recurses and finds repositories.
    </p>

    <div class="flex gap-2 mb-6">
        <input
            type="text"
            class="folder-input flex-1 min-w-0 px-3 py-2 text-sm font-mono text-surface-z9 bg-surface-z2 border border-surface-z3 rounded-md outline-none"
            bind:value={wizardState.roots.newPath}
            onkeydown={(e) => {
                if (e.key === "Enter") addRoot();
            }}
            placeholder="~/Developer"
            disabled={adding}
        />
        <button class="btn-solid" onclick={addRoot} disabled={adding}>
            {adding ? "Adding…" : "Add"}
        </button>
    </div>

    <div class="flex flex-col gap-3 mb-8">
        {#each roots as r (r.id)}
            <div
                class="flex items-center gap-3 px-5 py-4 bg-surface-z2 rounded-lg"
            >
                <span class="text-xs text-surface-z5">&#9656;</span>
                <div class="flex-1 min-w-0">
                    <div
                        class="text-sm font-mono whitespace-nowrap overflow-hidden text-ellipsis"
                    >
                        {r.path}
                    </div>
                    {#if r.repos_found > 0}
                        <div class="text-xs text-surface-z6 mt-0.5">
                            {r.repos_found} repositories found
                        </div>
                    {/if}
                </div>
                {#if r.status === "watching"}
                    <span
                        class="chip-watching text-xs text-success-z5 border border-success-z2 bg-success-z1 rounded-md px-2 py-0.5 whitespace-nowrap"
                        >watching</span
                    >
                {:else}
                    <span
                        class="text-xs text-surface-z6 border border-surface-z3 rounded-md px-2 py-0.5 whitespace-nowrap"
                        >recursive</span
                    >
                {/if}
                <button
                    class="text-base text-surface-z5 bg-none border-none cursor-pointer px-1 leading-none hover:text-primary-z5"
                    onclick={() => removeRoot(r.id)}>×</button
                >
            </div>
        {/each}
    </div>

    <p class="text-sm text-surface-z6">
        You can manage roots and exclusions later from Settings.
    </p>
</div>

<style>
    /* Folder input pseudo-classes */
    .folder-input::placeholder {
        color: oklch(var(--color-surface-z5) / 1);
    }
    .folder-input:focus {
        border-color: oklch(var(--color-surface-z6) / 1);
    }
</style>
