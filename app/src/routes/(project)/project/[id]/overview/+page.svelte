<script lang="ts">
    let { data } = $props();
    let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
    let ftrPrev = $derived(
        Math.round((data.ftrMetrics?.ftr14dPrev ?? 0) * 100),
    );
    let ftrDelta = $derived(ftr - ftrPrev);
</script>

<div class="px-6 py-6 max-w-[800px]">
    <!-- Hero: top recommendation -->
    {#if data.topRecommendation}
        <div class="bg-surface-z2 rounded-lg p-5 mb-5">
            <span class="text-2xs opacity-60 block mb-1.5"
                >Top recommendation</span
            >
            <p class="text-base font-semibold m-0 mb-2">
                {data.topRecommendation.title}
            </p>
            <span class="text-2xs px-2 py-0.5 rounded-lg bg-surface-z3"
                >{data.topRecommendation.urgency}</span
            >
        </div>
    {:else}
        <div class="bg-surface-z2 rounded-lg p-5 mb-5 opacity-50">
            <p class="text-ui opacity-50 m-0">No pending recommendations.</p>
        </div>
    {/if}

    <!-- Stat blocks -->
    <div class="flex gap-4 mb-6 flex-wrap">
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block">{ftr}%</span>
            <span class="text-2xs opacity-50 block">FTR 14d</span>
            {#if ftrDelta !== 0}
                <span
                    class="stat-delta text-xs"
                    class:pos={ftrDelta > 0}
                    class:neg={ftrDelta < 0}
                >
                    {ftrDelta > 0 ? "+" : ""}{ftrDelta}%
                </span>
            {/if}
        </div>
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block"
                >{data.ftrMetrics?.sessions7d ?? 0}</span
            >
            <span class="text-2xs opacity-50 block">Sessions 7d</span>
        </div>
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block">{data.memoryCount}</span>
            <span class="text-2xs opacity-50 block">Memories</span>
            {#if data.memoriesPendingShare > 0}
                <span
                    class="text-2xs bg-primary-z5 text-surface-z1 px-1.5 py-0.5 rounded-lg"
                    >{data.memoriesPendingShare} to share</span
                >
            {/if}
        </div>
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block">{data.repos.length}</span>
            <span class="text-2xs opacity-50 block">Repos</span>
        </div>
    </div>

    <!-- Recent sessions -->
    {#if data.recentSessions.length > 0}
        <section>
            <h3 class="text-ui font-semibold m-0 mb-2.5 opacity-70">
                Recent sessions
            </h3>
            {#each data.recentSessions as session (session.id)}
                <div
                    class="session-row flex justify-between py-1.5 border-b border-surface-z2 text-ui"
                >
                    <span>{session.task}</span>
                    <span
                        class="ftr-mark"
                        class:ftr-pass={session.ftr}
                        class:ftr-fail={session.ftr === false}
                    >
                        {session.ftr === true
                            ? "✓"
                            : session.ftr === false
                              ? "✗"
                              : "—"}
                    </span>
                </div>
            {/each}
        </section>
    {/if}
</div>

<style>
    .stat-delta.pos {
        color: oklch(var(--color-success-z5) / 1);
    }
    .stat-delta.neg {
        color: oklch(var(--color-primary-z5) / 1);
    }
    .ftr-mark.ftr-pass {
        color: oklch(var(--color-success-z5) / 1);
    }
    .ftr-mark.ftr-fail {
        color: oklch(var(--color-primary-z5) / 1);
    }
    .session-row:last-child {
        border-bottom: none;
    }
</style>
