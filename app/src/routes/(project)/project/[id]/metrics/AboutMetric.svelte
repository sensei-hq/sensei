<script lang="ts">
    import { Eyebrow, Kanji } from '$lib/components';
    import type { MetricAbout, TextSegment } from '$lib/metrics/metric-view.js';

    let {
        about,
        howToReadSegments = [],
        projectId = '',
    }: { about: MetricAbout; howToReadSegments?: TextSegment[]; projectId?: string } = $props();

    // Only the fields that carry copy — a metric with no formula shows two rows,
    // never an empty "How it's computed". `metricAbout` returns null when nothing
    // is present, so `about` here always has at least one row.
    const rows = $derived(
        [
            { key: 'purpose', label: 'What it tells you', text: about.purpose, mono: false },
            { key: 'how', label: 'How to read it', text: about.howToRead, mono: false },
            { key: 'formula', label: 'How it’s computed', text: about.formula ?? '', mono: true },
        ].filter((r) => r.text.trim().length > 0),
    );
</script>

<section data-component="metric-about" class="flex flex-col gap-3">
    <div class="flex items-center gap-2">
        <Kanji char="解" size="sm" tone="accent" />
        <Eyebrow>About this metric</Eyebrow>
    </div>
    <dl class="grid grid-cols-1 sm:grid-cols-[168px_1fr] gap-x-6 gap-y-3 m-0">
        {#each rows as r (r.key)}
            <dt class="text-xs text-ink-faint pt-0.5">{r.label}</dt>
            <dd
                data-row={r.key}
                class="m-0 text-sm leading-relaxed text-pretty {r.mono
                    ? 'mono text-ink-mute'
                    : 'text-ink-soft'}"
            >{#if r.key === 'how' && howToReadSegments.length}{#each howToReadSegments as seg, si (si)}{#if seg.key && projectId}<a
                            data-companion={seg.key}
                            href={`/project/${projectId}/metrics/${seg.key}`}
                            class="text-accent no-underline hover:underline">{seg.text}</a>{:else}{seg.text}{/if}{/each}{:else}{r.text}{/if}</dd>
        {/each}
    </dl>
</section>
