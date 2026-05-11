<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import type { SessionData } from "$lib/types.js";

    type Session = SessionData["sessions"][number];

    let sessions = $state<Session[]>([]);
    let stats = $state<{
        count: number;
        ftr: number;
        corrections: number;
        projects: number;
    }>({
        count: 0,
        ftr: 0,
        corrections: 0,
        projects: 0,
    });
    let loading = $state(true);
    let filter = $state<"all" | "completed" | "corrected" | "abandoned">("all");

    onMount(async () => {
        const api = senseiApi(appState.port);
        const data = await api.getSessions();
        sessions = data.sessions ?? [];
        if (data.stats) {
            const s = data.stats as Record<string, number>;
            stats = {
                count: s.total_sessions ?? sessions.length,
                ftr: s.ftr_rate ?? 0,
                corrections: s.total_corrections ?? 0,
                projects: s.project_count ?? 0,
            };
        }
        loading = false;
    });

    let filtered = $derived(
        filter === "all"
            ? sessions
            : sessions.filter((s) => s.outcome === filter),
    );

    function formatTime(iso: string): string {
        if (!iso) return "";
        const d = new Date(iso);
        return (
            d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
            " · " +
            d.toLocaleTimeString("en-US", {
                hour: "numeric",
                minute: "2-digit",
            })
        );
    }
</script>

<div class="max-w-[820px] mx-auto px-12 py-12 pb-16">
    <div class="mb-8">
        <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
            Sessions
        </p>
        <h1 class="display text-2xl font-normal m-0">刻 Sessions</h1>
    </div>

    <!-- Stats strip -->
    <div
        class="flex gap-8 mb-7 px-6 py-5 bg-surface-z2 border border-surface-z3 rounded-lg"
    >
        {#each [{ value: stats.count, label: "sessions (7d)" }, { value: Math.round(stats.ftr * 100) + "%", label: "FTR" }, { value: stats.corrections, label: "corrections" }, { value: stats.projects, label: "projects" }] as stat}
            <div class="flex flex-col gap-0.5">
                <span class="display text-2xl font-normal">{stat.value}</span>
                <span class="text-2xs text-surface-z6">{stat.label}</span>
            </div>
        {/each}
    </div>

    <!-- Filters -->
    <div class="flex gap-1.5 mb-6">
        {#each ["all", "completed", "corrected", "abandoned"] as f}
            <button
                class="filter-chip px-3.5 py-1.25 rounded-full border border-surface-z3 bg-transparent text-xs cursor-pointer text-surface-z7 capitalize"
                class:active={filter === f}
                onclick={() => (filter = f as any)}>{f}</button
            >
        {/each}
    </div>

    <!-- Sessions list -->
    {#if loading}
        <p class="text-ui text-surface-z6">Loading sessions...</p>
    {:else if filtered.length === 0}
        <EmptyState
            kanji="刻"
            title="No sessions yet."
            description="Start a session with your assistant. Each session becomes a moment of learning."
        />
    {:else}
        <div class="flex flex-col gap-px">
            {#each filtered as session (session.id)}
                <div
                    class="session-row flex items-center gap-3 px-4 py-3 rounded-md transition-colors duration-100"
                >
                    <span
                        class="ftr-dot w-1.75 h-1.75 rounded-full shrink-0"
                        class:green={session.ftr === 1}
                        class:amber={session.ftr !== 1}
                    ></span>
                    <div class="flex-1 flex flex-col gap-0.5 min-w-0">
                        <span
                            class="text-ui text-surface-z9 whitespace-nowrap overflow-hidden text-ellipsis"
                            >{session.task || session.id.slice(0, 8)}</span
                        >
                        <span class="text-2xs text-surface-z6"
                            >{session.project || "unknown"}</span
                        >
                    </div>
                    <span
                        class="text-2xs text-surface-z6 capitalize w-20 text-right"
                        >{session.outcome ?? "—"}</span
                    >
                    <span class="text-2xs text-surface-z5 w-[140px] text-right"
                        >{formatTime(session.startedAt)}</span
                    >
                </div>
            {/each}
        </div>
    {/if}
</div>

<style>
    .filter-chip:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .filter-chip.active {
        background: oklch(var(--color-surface-z9) / 1);
        color: oklch(var(--color-surface-z1) / 1);
        border-color: oklch(var(--color-surface-z9) / 1);
    }

    .session-row:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }

    .ftr-dot.green {
        background: oklch(var(--color-success-z5) / 1);
    }
    .ftr-dot.amber {
        background: oklch(var(--color-warning-z5) / 1);
    }
</style>
