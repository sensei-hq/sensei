<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import {
        stageIndex,
        nextStagePath,
        prevStagePath,
    } from "./stages.js";
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { loadWizardData } from "$lib/setup/loaders.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { StatusDot } from "$lib/components";

    let { children } = $props();

    const stages = $derived(wizardState.stages);
    const currentIdx = $derived(stageIndex(page.url.pathname));
    const stage = $derived(stages[currentIdx]);
    const isFirst = $derived(currentIdx === 0);
    const isLast = $derived(currentIdx === stages.length - 1);
    const total = $derived(stages.length);
    const canAdvance = $derived(wizardState.canAdvance(stage?.id ?? ""));
    let committing = $state(false);
    let loaded = $state(false);

    // Drive the transient `active` flag from the current route so both the
    // rail and the header read the same shape per stage.
    $effect(() => {
        if (stage) wizardState.setActive(stage.id);
    });

    onMount(async () => {
        const data = await loadWizardData(appState.port);
        await wizardState.hydrate(data);
        loaded = true;
        // Daemon-canonical: if setup is already complete, the user was
        // dropped here during the cold-start race (appState wasn't loaded
        // when reroute decided). Send them to the observatory now.
        if (wizardState.isOk) goto("/");
    });

    async function next() {
        if (committing) return;
        if (!canAdvance) return;

        committing = true;
        if (isLast) {
            await wizardState.commitStage("done");
            committing = false;
            goto("/");
            return;
        }
        const ok = await wizardState.commitStage(stage.id);
        committing = false;
        if (ok) {
            const path = nextStagePath(page.url.pathname);
            if (path) goto(path);
        }
    }

    function back() {
        const path = prevStagePath(page.url.pathname);
        if (path) goto(path);
    }
</script>

<div
    class="w-full h-screen flex flex-col bg-surface-z1 text-surface-z9 overflow-hidden"
>
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 grid grid-cols-[260px_1fr] min-h-0">
        <!-- Rail -->
        <aside
            class="flex flex-col px-6 py-6 border-r border-surface-z2 bg-surface-z2 overflow-hidden"
        >
            <div class="flex items-baseline gap-2 mb-7">
                <span class="kanji text-[22px] text-primary-z6">先生</span>
                <span class="display text-lg">Sensei</span>
            </div>

            <div class="text-xs tracking-wide text-surface-z6 uppercase mb-3.5">
                Setup
            </div>

            <div class="flex flex-col">
                {#each stages as s (s.id)}
                    {@const isDone = s.status === "done"}
                    {@const isNavigable = isDone || s.active}
                    <button
                        data-rail-item
                        class="grid grid-cols-[24px_1fr_14px] px-2 py-1 gap-2.5 items-center rounded-md text-left border border-transparent text-surface-z5 cursor-default transition-all duration-fast text-sm"
                        class:active={s.active}
                        class:done={isDone}
                        onclick={() => {
                            if (isNavigable) goto(s.path);
                        }}
                        disabled={!isNavigable}
                    >
                        <span
                            class="rail-kanji kanji text-sm text-center text-surface-z5"
                            class:active={s.active}
                            class:done={isDone}
                            class:text-primary-z6={s.active}>{s.icon}</span
                        >
                        <div class="overflow-hidden">
                            <div class:leading-tight={s.active}>
                                {s.title}
                            </div>
                            {#if s.active}
                                <div
                                    class="mono text-xs text-surface-z6 mt-0.5"
                                >
                                    {s.brief}
                                </div>
                            {/if}
                        </div>
                        <span
                            class="text-xs text-center leading-none text-success-z6 opacity-0 transition-opacity duration-fast"
                            class:opacity-100={isDone}>✓</span
                        >
                    </button>
                {/each}
            </div>

            <div class="mt-auto border-t border-surface-z2 pt-3">
                <div class="flex items-center gap-2.5">
                    <StatusDot status="ok" />
                    <div class="text-xs text-surface-z7 leading-snug">
                        <div
                            class="tracking-wide uppercase text-xs text-surface-z6"
                        >
                            Services
                        </div>
                        <div class="mt-0.5">all green</div>
                    </div>
                </div>
            </div>
        </aside>

        <!-- Content -->
        <div class="flex flex-col min-h-0">
            {#if stage?.id !== "welcome"}
                <div
                    class="shrink-0 px-16 pt-7 pb-6 border-b border-surface-z3 bg-surface-z1 relative z-1"
                >
                    <div
                        class="text-xs text-surface-z6 tracking-wide uppercase mb-2"
                    >
                        <span class="kanji text-primary-z6 mr-1.5 opacity-60"
                            >{stage.icon}</span
                        >Step
                    </div>
                    <h1
                        class="display text-4xl font-light tracking-tight m-0 mb-1.5"
                    >
                        {stage.title}
                    </h1>
                    <p class="text-sm text-surface-z6 m-0">
                        {stage.description}
                    </p>
                </div>
            {/if}

            <div
                class="flex-1 overflow-y-auto px-16 relative"
                class:py-8={stage?.id !== "welcome"}
                class:pt-11={stage?.id === "welcome"}
                class:pb-8={stage?.id === "welcome"}
            >
                {#if stage?.watermark}
                    <span
                        class="watermark kanji absolute right-16 bottom-8 text-4xl text-primary-z6 opacity-[0.035] leading-none select-none pointer-events-none z-0"
                        >{stage.icon}</span
                    >
                {/if}
                {@render children()}
            </div>

            <!-- Bottom nav -->
            <div
                class="flex items-center gap-5 px-16 py-3.5 border-t border-surface-z2 bg-surface-z1 shrink-0"
            >
                <div class="flex items-baseline gap-3">
                    <span
                        class="text-xs tracking-wide text-surface-z6 uppercase"
                    >
                        {String(currentIdx + 1).padStart(2, "0")}
                        <span class="text-surface-z5">/ {total}</span>
                    </span>
                    <span class="text-sm text-surface-z7">{stage.title}</span>
                </div>

                <div class="flex-1 flex gap-1 items-center">
                    {#each Array(total) as _, i}
                        <span
                            class="flex-1 h-0.5 rounded-sm bg-surface-z3 transition-colors duration"
                            class:bg-surface-z9={i <= currentIdx}
                        ></span>
                    {/each}
                </div>

                <div class="flex gap-2 items-center">
                    <button
                        class="btn-back text-xs text-surface-z7 px-3.5 py-1 leading-3 bg-none border-none cursor-pointer"
                        onclick={back}
                        disabled={isFirst}
                    >
                        ← Back
                    </button>
                    <button
                        class="btn-primary text-sm leading-3 bg-surface-z9 text-surface-z1 px-6 py-2.5 rounded-md border-none tracking-normal cursor-pointer"
                        onclick={next}
                        disabled={!canAdvance || committing}
                    >
                        {#if committing}
                            {stage?.id === "assistants"
                                ? "Configuring…"
                                : "Saving…"}
                        {:else if isLast}
                            Enter observatory →
                        {:else if stage?.id === "assistants"}
                            Configure &amp; Continue →
                        {:else}
                            Continue →
                        {/if}
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    /* Rail item states */
    /*.rail-item {
        padding: 7px 10px;
    }*/
    [data-rail-item]:not(:disabled) {
        cursor: pointer;
    }
    [data-rail-item].active {
        background: oklch(var(--color-surface-z1) / 1);
        border-color: oklch(var(--color-surface-z3) / 1);
        color: oklch(var(--color-surface-z9) / 1);
    }
    [data-rail-item].done {
        color: oklch(var(--color-surface-z7) / 1);
    }

    /* Rail kanji states */
    .rail-kanji.done {
        color: oklch(var(--color-surface-z7) / 1);
    }

    /* Back/primary button disabled states */
    .btn-back:disabled {
        color: oklch(var(--color-surface-z5) / 1);
        cursor: default;
    }
    .btn-primary:hover:not(:disabled) {
        opacity: 0.9;
    }
    .btn-primary:disabled {
        background: oklch(var(--color-surface-z3) / 1);
        color: oklch(var(--color-surface-z6) / 1);
        cursor: default;
    }
</style>
