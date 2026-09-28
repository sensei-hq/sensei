<script lang="ts">
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { RootExclusions } from "./root-exclusions.svelte.js";

    const roots = $derived(wizardState.roots.roots);
    let adding = $state(false);

    async function addRoot() {
        const trimmed = wizardState.roots.newPath.trim();
        if (!trimmed || roots.some((r) => r.path === trimmed) || adding) return;

        adding = true;
        const api = senseiApi(appState.port);
        const result = await api.addWatchRoot(trimmed);

        if (result.ok) {
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

    // ── Exclusions ────────────────────────────────────────────────────────────
    // The store, the endpoint and the watcher check all existed; nothing reached
    // them. MEASURED: one project carried 1,230 folders, 1,211 under an unpacked
    // Node source tarball — no `node_modules`, so EXCLUDE_DIRS does not apply, and
    // no `.gitignore` in a non-git folder to bound it either.
    //
    // One controller per root, built lazily and kept, so each row owns its own
    // in-flight and error state rather than sharing one.
    const controllers = new Map<string, RootExclusions>();
    function exclusionsFor(r: { id: string; path: string; excluded?: string[] }) {
        let c = controllers.get(r.id);
        if (!c) {
            c = new RootExclusions(
                r.id,
                r.path,
                r.excluded ?? [],
                senseiApi(appState.port),
            );
            controllers.set(r.id, c);
        }
        return c;
    }

    /** Which root's exclusion editor is open. One at a time — the list is a
     *  secondary control and expanding every row would bury the roots. */
    let openFor = $state<string | null>(null);
    let draft = $state('');

    async function addExclusion(r: { id: string; path: string; excluded?: string[] }) {
        const c = exclusionsFor(r);
        if (await c.add(draft)) draft = '';
    }
</script>

<div class="max-w-[780px]">
    <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
        Where your work lives. Sensei recurses each root and finds repositories.
    </p>

    <div class="flex gap-2 mb-6">
        <input
            type="text"
            class="folder-input flex-1 min-w-0 px-3 py-2 text-sm font-mono text-ink bg-paper-mute border border-paper-edge rounded-md outline-none"
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
                data-component="root-item"
                class="flex items-center gap-3 px-5 py-4 bg-paper-mute rounded-lg"
            >
                <span class="text-xs text-ink-soft">&#9656;</span>
                <div class="flex-1 min-w-0">
                    <div
                        data-component="root-path"
                        class="text-sm font-mono whitespace-nowrap overflow-hidden text-ellipsis"
                    >
                        {r.path}
                    </div>
                    {#if r.repos_found > 0}
                        <div class="text-xs text-ink-soft mt-0.5">
                            {r.repos_found} repositories found
                        </div>
                    {/if}
                </div>
                {#if r.status === "watching"}
                    <span
                        class="chip-watching text-xs text-success border border-success bg-success-soft rounded-md px-2 py-0.5 whitespace-nowrap"
                        >watching</span
                    >
                {:else}
                    <span
                        class="text-xs text-ink-soft border border-paper-edge rounded-md px-2 py-0.5 whitespace-nowrap"
                        >recursive</span
                    >
                {/if}
                <button
                    data-testid="root-skips"
                    class="text-xs text-ink-soft border border-paper-edge rounded-md px-2 py-0.5 whitespace-nowrap cursor-pointer hover:text-ink"
                    onclick={() => {
                        openFor = openFor === r.id ? null : r.id;
                        draft = "";
                    }}
                >
                    <!-- From the controller, not the loaded root: it is seeded from
                         `r.excluded` and is authoritative afterwards, so the count
                         follows an add or a remove instead of going stale. -->
                    {exclusionsFor(r).excluded.length} skipped
                </button>
                <button
                    data-testid="root-remove"
                    class="text-base text-ink-soft bg-none border-none cursor-pointer px-1 leading-none hover:text-accent"
                    onclick={() => removeRoot(r.id)}>×</button
                >
            </div>

            {#if openFor === r.id}
                {@const ex = exclusionsFor(r)}
                <div
                    data-component="root-exclusions"
                    data-root={r.id}
                    class="ml-8 -mt-1 px-5 py-4 bg-paper-soft rounded-lg"
                >
                    <p class="text-xs text-ink-soft m-0 mb-3 max-w-[560px] leading-normal">
                        Folders sensei will not walk under this root. A plain name skips
                        every folder with that name; a full path skips one subtree.
                    </p>

                    {#if ex.excluded.length === 0}
                        <p class="text-xs text-ink-mute m-0 mb-3" data-empty>
                            Nothing skipped. Only .gitignore and the built-in list
                            (node_modules, target, dist…) bound this root — a folder that
                            is not a git repository has neither.
                        </p>
                    {:else}
                        <div class="flex flex-col gap-1 mb-3" data-exclusion-list>
                            {#each ex.excluded as value (value)}
                                <div
                                    class="flex items-baseline gap-3"
                                    data-exclusion={value}
                                >
                                    <span class="text-xs font-mono text-ink truncate">{value}</span>
                                    <span class="text-xs text-ink-faint flex-1 min-w-0 truncate">
                                        {ex.describe(value)}
                                    </span>
                                    <button
                                        class="text-xs text-ink-soft bg-none border-none cursor-pointer hover:text-accent"
                                        disabled={ex.saving}
                                        onclick={() => ex.remove(value)}>remove</button
                                    >
                                </div>
                            {/each}
                        </div>
                    {/if}

                    <div class="flex gap-2">
                        <input
                            type="text"
                            data-testid="exclusion-input"
                            class="folder-input flex-1 min-w-0 px-3 py-1.5 text-xs font-mono text-ink bg-paper-mute border border-paper-edge rounded-md outline-none"
                            bind:value={draft}
                            onkeydown={(e) => {
                                if (e.key === "Enter") addExclusion(r);
                            }}
                            placeholder="node_modules  ·  {r.path}/some/subtree"
                            disabled={ex.saving}
                        />
                        <button
                            class="btn-solid"
                            disabled={ex.saving || !draft.trim()}
                            onclick={() => addExclusion(r)}>Skip</button
                        >
                    </div>

                    {#if draft.trim() && !ex.error}
                        <!-- Says what it will do BEFORE it is saved: the two forms look
                             almost identical typed and behave very differently. -->
                        <p class="text-xs text-ink-faint mt-2 mb-0" data-preview>
                            {ex.describe(draft.trim())}
                        </p>
                    {/if}
                    {#if ex.error}
                        <p class="text-xs text-danger mt-2 mb-0" data-error>{ex.error}</p>
                    {/if}
                </div>
            {/if}
        {/each}
    </div>
</div>

<style>
    /* Folder input pseudo-classes */
    .folder-input::placeholder {
        color: var(--ink-soft);
    }
    .folder-input:focus {
        border-color: var(--ink-soft);
    }
</style>
