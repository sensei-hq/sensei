<script lang="ts">
    import { Sparkline } from '@rokkit/chart';

    let { data } = $props();
    let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
    let ftrPrev = $derived(
        Math.round((data.ftrMetrics?.ftr14dPrev ?? 0) * 100),
    );
    let ftrDelta = $derived(ftr - ftrPrev);

    function signalStatus(value: number | null, threshold: number): 'pass' | 'warn' | 'fail' | 'unknown' {
        if (value == null) return 'unknown';
        if (value >= threshold) return 'pass';
        if (value >= threshold * 0.7) return 'warn';
        return 'fail';
    }
</script>

<div class="px-6 py-6 max-w-[860px]">
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

    <!-- Stat blocks + sparkline -->
    <div class="flex gap-4 mb-6 flex-wrap items-end">
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
        {#if data.ftrDaily.length >= 2}
            <div class="bg-surface-z2 rounded-lg p-4">
                <Sparkline data={data.ftrDaily} field="ftr_rate" width={120} height={32} />
                <span class="text-2xs opacity-50 block mt-1">14d trend</span>
            </div>
        {/if}
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block"
                >{data.ftrMetrics?.sessions7d ?? 0}</span
            >
            <span class="text-2xs opacity-50 block">Sessions 7d</span>
        </div>
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block">{data.memoryCount}</span>
            <span class="text-2xs opacity-50 block">Memories</span>
        </div>
        <div class="bg-surface-z2 rounded-lg p-4 min-w-[100px]">
            <span class="text-3xl font-bold block">{data.repos.length}</span>
            <span class="text-2xs opacity-50 block">Repos</span>
        </div>
    </div>

    <!-- Quality signals -->
    {#if data.qualitySignals}
        {@const s = data.qualitySignals}
        {@const ftrSig = signalStatus(s.ftr_7d, 0.7)}
        <div class="flex gap-2 mb-6 text-xs flex-wrap">
            <span class="signal" class:signal-pass={ftrSig === 'pass'} class:signal-warn={ftrSig === 'warn'} class:signal-fail={ftrSig === 'fail'}>
                {ftrSig === 'pass' ? '✓' : ftrSig === 'warn' ? '⚠' : ftrSig === 'fail' ? '✗' : '—'}
                Zero-errors: {Math.round(s.ftr_7d * 100)}% sessions passed first try
            </span>
            {#if s.pattern_compliance != null}
                {@const patSig = signalStatus(s.pattern_compliance, 0.8)}
                <span class="signal" class:signal-pass={patSig === 'pass'} class:signal-warn={patSig === 'warn'} class:signal-fail={patSig === 'fail'}>
                    {patSig === 'pass' ? '✓' : patSig === 'warn' ? '⚠' : '✗'}
                    Pattern compliance: {Math.round(s.pattern_compliance * 100)}%
                </span>
            {/if}
            <span class="signal" class:signal-pass={s.open_drift_count === 0} class:signal-warn={s.open_drift_count > 0 && s.open_drift_count <= 3} class:signal-fail={s.open_drift_count > 3}>
                {s.open_drift_count === 0 ? '✓' : '⚠'}
                Doc drift: {s.open_drift_count} open
            </span>
            {#if s.test_pass_rate != null}
                {@const testSig = signalStatus(s.test_pass_rate, 0.95)}
                <span class="signal" class:signal-pass={testSig === 'pass'} class:signal-warn={testSig === 'warn'} class:signal-fail={testSig === 'fail'}>
                    {testSig === 'pass' ? '✓' : testSig === 'warn' ? '⚠' : '✗'}
                    Tests: {Math.round(s.test_pass_rate * 100)}% pass
                </span>
            {/if}
        </div>
    {/if}

    <div class="grid grid-cols-[1.3fr_1fr] gap-8">
        <div>
            <!-- Recent sessions -->
            {#if data.recentSessions.length > 0}
                <section class="mb-6">
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

            <!-- Hotspots -->
            {#if data.hotspots.length > 0}
                <section>
                    <h3 class="text-ui font-semibold m-0 mb-2.5 opacity-70">Hotspots</h3>
                    {#each data.hotspots.slice(0, 5) as h}
                        <div class="flex justify-between py-1.5 border-b border-surface-z2 text-ui">
                            <span class="mono text-xs truncate flex-1 mr-3">{h.file_path}</span>
                            <span class="mono text-xs opacity-50 shrink-0">
                                {h.correction_count > 0 ? `${h.correction_count}× rework` : `${h.edit_count} edits`}
                            </span>
                        </div>
                    {/each}
                </section>
            {/if}
        </div>

        <!-- Teachings (right column) -->
        <div>
            {#if data.teachings.length > 0}
                <section>
                    <h3 class="text-ui font-semibold m-0 mb-2.5 opacity-70">Adopted teachings</h3>
                    {#each data.teachings as t (t.id)}
                        <div class="teaching-card py-3 px-3.5 mb-2 rounded-md bg-surface-z2 border border-surface-z3">
                            <p class="text-sm m-0 leading-snug">{t.name}</p>
                            <p class="text-2xs text-surface-z6 m-0 mt-1">
                                {t.family ?? 'pattern'} · {t.instance_count} places
                            </p>
                        </div>
                    {/each}
                </section>
            {/if}
        </div>
    </div>
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
    .signal {
        padding: 3px 10px;
        border-radius: 4px;
        background: oklch(var(--color-surface-z2) / 1);
        white-space: nowrap;
    }
    .signal-pass { color: oklch(var(--color-success-z5) / 1); }
    .signal-warn { color: oklch(0.75 0.15 75); }
    .signal-fail { color: oklch(var(--color-primary-z5) / 1); }
    .teaching-card {
        border-left: 2px solid oklch(var(--color-primary-z5) / 1);
    }
</style>
