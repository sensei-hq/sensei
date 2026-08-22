<script lang="ts">
    import { Eyebrow, Kanji } from '$lib/components';
    import type { MetricAbout, TextSegment } from '$lib/metrics/metric-view.js';

    // What the metric means, as primary content rather than reference material.
    // The mockups (03-ev.png, 04-ev.png) lay this out as labelled columns under
    // the hero value — "WHAT IT MEASURES" beside "HOW TO READ IT", with the
    // arithmetic in its own block — because a number nobody can interpret is the
    // "no why" gap the design set out to close (annotation on 01-ev.png). It used
    // to be a label/value <dl> behind an info popover.
    let {
        about,
        howToReadSegments = [],
        projectId = '',
    }: { about: MetricAbout; howToReadSegments?: TextSegment[]; projectId?: string } = $props();

    const purpose = $derived(about.purpose.trim());
    const howToRead = $derived(about.howToRead.trim());
    const formula = $derived((about.formula ?? '').trim());
</script>

<section
    data-component="metric-about"
    class="rounded-md border border-paper-edge bg-paper p-4 flex flex-col gap-4"
>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-4">
        {#if purpose}
            <div class="flex flex-col gap-2">
                <Eyebrow>What it measures</Eyebrow>
                <p
                    data-row="purpose"
                    class="m-0 text-sm leading-relaxed text-ink-soft text-pretty"
                >{purpose}</p>
            </div>
        {/if}

        {#if howToRead}
            <div class="flex flex-col gap-2">
                <Eyebrow>How to read it</Eyebrow>
                <p
                    data-row="how"
                    class="m-0 text-sm leading-relaxed text-ink-soft text-pretty"
                >{#if howToReadSegments.length}{#each howToReadSegments as seg, si (si)}{#if seg.key && projectId}<a
                                data-companion={seg.key}
                                href={`/project/${projectId}/metrics/${seg.key}`}
                                class="text-accent no-underline hover:underline">{seg.text}</a>{:else}{seg.text}{/if}{/each}{:else}{howToRead}{/if}</p>
            </div>
        {/if}
    </div>

    {#if formula}
        <div class="flex flex-col gap-2 border-t border-paper-edge pt-4">
            <div class="flex items-center gap-2">
                <Kanji char="式" size="sm" tone="accent" />
                <Eyebrow>How this number was calculated</Eyebrow>
            </div>
            <code
                data-row="formula"
                class="mono block m-0 rounded border border-paper-edge bg-paper-soft px-3 py-2 text-xs leading-relaxed text-ink-mute"
            >{formula}</code>
        </div>
    {/if}
</section>
