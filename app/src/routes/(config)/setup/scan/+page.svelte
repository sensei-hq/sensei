<script lang="ts">
    import { onMount } from "svelte";
    import { scanState } from "$lib/scan-state.svelte.js";

    const projects = scanState.projects;
    const activities = scanState.activities;

    // The scan is owned by this stage. Auto-engage on mount so the page
    // reflects current daemon state without a manual button — the
    // singleton's idempotency guard makes repeat calls cheap.
    onMount(() => { void scanState.start(); });

    // Task-status colors used by the right-panel folder list. Status
    // transitions queued → indexing → indexed (or → failed), each shown
    // with its own colour so the user can see live state at a glance.
    const STATUS_COLORS: Record<string, string> = {
        discovered: "oklch(var(--color-surface-z6) / 1)",
        queued:     "oklch(var(--color-warning-z5) / 1)",
        indexing:   "oklch(var(--color-primary-z5) / 1)",
        indexed:    "oklch(var(--color-success-z5) / 1)",
        failed:     "oklch(var(--color-danger-z5) / 1)",
    };
</script>

<div class="max-w-[960px]">
    {#if !scanState.started}
        <div
            class="flex flex-col items-center text-center gap-6 bg-surface-z2 rounded-lg px-10 py-12"
        >
            <div class="kanji text-4xl text-primary-z5 opacity-25 leading-none">
                探
            </div>
            <p
                class="text-base text-surface-z7 max-w-[440px] leading-normal m-0"
            >
                The daemon will recurse your folders, identify folders, and
                extract the code graph.
            </p>
            <button class="btn-solid" onclick={() => scanState.start()}>Begin scan →</button>
        </div>
    {:else}
        {#if scanState.error}
            <div
                class="mb-6 p-4 rounded-md border border-danger-z5 bg-surface-z2 flex items-start justify-between gap-4"
            >
                <div>
                    <div class="text-sm font-semibold text-danger-z5">
                        Scan failed
                    </div>
                    <div class="text-xs text-surface-z7 mt-1 font-mono">
                        {scanState.error}
                    </div>
                </div>
                <button class="btn-outline shrink-0" onclick={() => scanState.start()}>
                    Retry
                </button>
            </div>
        {:else if !scanState.daemonReachable}
            <div
                class="mb-4 p-2.5 rounded-md bg-surface-z2 text-xs text-warning-z5"
            >
                Daemon unreachable — retrying…
            </div>
        {/if}

        <!-- Stats bar -->
        <div
            class="grid grid-cols-4 gap-4 mb-6 pb-6 border-b border-surface-z2"
        >
            {#each [{ value: scanState.rootCount, label: "ROOTS" }, { value: activities.discovered, label: "DISCOVERED" }, { value: projects.pendingFolders, label: "QUEUED" }, { value: projects.readyFolders, label: "PROCESSED" }] as stat}
                <div class="text-center">
                    <div class="display text-3xl leading-tight">
                        {stat.value}
                    </div>
                    <div class="text-xs tracking-wide text-surface-z6 mt-1">
                        {stat.label}
                    </div>
                </div>
            {/each}
        </div>

        <div class="grid grid-cols-[1fr_340px] gap-6">
            <!-- Left: Project cards -->
            <div class="flex flex-col gap-4">
                {#each projects.sortedItems as proj (proj.id)}
                    {@const path = projects.projectPath(proj)}
                    {@const folderCount = proj.folders.length}
                    {@const readyCount = proj.folders.filter(
                        (f) => f.status === "indexed",
                    ).length}
                    {@const totalFiles = proj.folders.reduce(
                        (s, f) => s + f.filesTotal,
                        0,
                    )}
                    {@const completedFiles = proj.folders.reduce(
                        (s, f) => s + f.filesCompleted,
                        0,
                    )}

                    <div
                        class="bg-surface-z2 rounded-lg p-5 border border-surface-z3"
                    >
                        <div class="flex justify-between items-start mb-3">
                            <div>
                                <div class="text-base font-semibold">
                                    {proj.name}
                                </div>
                                <div class="text-xs text-surface-z6 mt-0.5">
                                    {path} · {folderCount} folders · {readyCount}
                                    ready
                                </div>
                            </div>
                            <span
                                class="text-xs tracking-wide text-surface-z6 font-semibold"
                                class:text-success-z5={proj.status === "active"}
                                >{proj.status.toUpperCase()}</span
                            >
                        </div>

                        {#if totalFiles > 0}
                            <div class="text-sm text-surface-z7 font-mono mb-3">
                                {completedFiles.toLocaleString()} / {totalFiles.toLocaleString()}
                            </div>
                        {/if}

                        <div class="flex flex-col gap-2">
                            {#each [...proj.folders].sort((a, b) => (b.lastUpdated ?? 0) - (a.lastUpdated ?? 0)) as f (f.id)}
                                <div class="flex items-center gap-3">
                                    <span
                                        class="text-sm font-medium min-w-[120px]"
                                        >{f.name}</span
                                    >
                                    {#if f.stack.length > 0}
                                        <span
                                            class="text-xs text-surface-z6 min-w-[80px]"
                                            >{f.stack.join(", ")}</span
                                        >
                                    {:else if f.filesTotal > 0}
                                        <span
                                            class="text-xs text-surface-z6 min-w-[80px]"
                                            >{f.filesTotal}f</span
                                        >
                                    {/if}
                                    {#if f.filesTotal > 0}
                                        <div
                                            class="flex-1 h-1 bg-surface-z3 rounded-sm overflow-hidden"
                                        >
                                            <div
                                                class="h-full bg-warning-z5 rounded-sm transition-[width] duration-slow"
                                                style="width: {(f.filesCompleted /
                                                    f.filesTotal) *
                                                    100}%"
                                            ></div>
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/each}

                {#if projects.items.length === 0 && scanState.started}
                    <div class="text-sm text-surface-z5 p-6 text-center">
                        Discovering projects...
                    </div>
                {/if}
            </div>

            <!-- Right: Task list (one row per folder, status evolves) -->
            <div
                class="bg-surface-z2 rounded-lg p-4 border border-surface-z3 max-h-[600px] flex flex-col"
            >
                <div
                    class="flex justify-between pb-3 border-b border-surface-z2 mb-3 text-xs text-surface-z6"
                >
                    <span class="mono">TASKS</span>
                    <span class="mono text-surface-z7"
                        >{projects.pendingFolders} active · {activities.totalElapsed.toFixed(
                            1,
                        )}s</span
                    >
                </div>
                <div class="flex-1 overflow-y-auto flex flex-col gap-0.5">
                    {#each projects.allFolders as f (f.id)}
                        <div
                            class="flex gap-3 text-xs py-1 items-baseline border-b border-surface-z2 last:border-b-0"
                        >
                            <span
                                class="font-medium flex-1 min-w-0 truncate"
                                title={f.path}>{f.name}</span
                            >
                            <span
                                class="text-xs min-w-[64px] text-right"
                                style="color: {STATUS_COLORS[f.status] ??
                                    'oklch(var(--color-surface-z6) / 1)'}"
                                >{f.status}</span
                            >
                            {#if f.filesTotal > 0}
                                <span
                                    class="mono text-xs text-surface-z6 min-w-[60px] text-right"
                                    >{f.filesCompleted}/{f.filesTotal}</span
                                >
                            {:else}
                                <span class="mono text-xs text-surface-z6 min-w-[60px] text-right">—</span>
                            {/if}
                        </div>
                    {/each}
                    {#if projects.allFolders.length === 0}
                        <div class="text-xs text-surface-z5 text-center p-4">
                            Waiting for the daemon to discover folders…
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</div>
