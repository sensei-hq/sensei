<script lang="ts">
    import { PageHeader } from '$lib/components';
</script>

<PageHeader kanji="助" eyebrow="Sensei" title="Help" variant="h1" />
<div class="max-w-[640px] px-8 pt-8 pb-16">

    <section class="mb-7">
        <h2 class="text-base font-bold m-0 mb-2.5">Quick Start</h2>
        <ol class="text-sm leading-loose pl-5 m-0">
            <li>
                <strong>Scan</strong> — Point Sensei at your project folders to index
                them.
            </li>
            <li>
                <strong>Watch</strong> — Sensei observes your AI coding sessions and
                learns your patterns.
            </li>
            <li>
                <strong>Teach</strong> — Adopt recommendations to improve your first-try
                rate.
            </li>
        </ol>
    </section>

    <section class="mb-7">
        <h2 class="text-base font-bold m-0 mb-2.5">Keyboard Shortcuts</h2>
        <table class="shortcuts border-collapse text-sm w-full">
            <thead>
                <tr>
                    <th
                        scope="col"
                        class="text-xs text-left py-1 pr-3 opacity-50 font-semibold"
                        >Shortcut</th
                    >
                    <th
                        scope="col"
                        class="text-xs text-left py-1 pr-3 opacity-50 font-semibold"
                        >Action</th
                    >
                </tr>
            </thead>
            <tbody>
                {#each [["⌘N", "New project"], ["⌘W", "Close window"], ["⌘,", "Preferences"], ["⌘\\", "Toggle sidebar"], ["⌘1", "Today"], ["⌘2", "Projects"], ["⌘3", "Sessions"], ["⌘0", "Show Observatory"], ["⌘?", "This help window"]] as [shortcut, action]}
                    <tr>
                        <td
                            class="shortcuts-row py-1 pr-3 font-mono text-sm font-semibold min-w-[60px]"
                            >{shortcut}</td
                        >
                        <td class="shortcuts-row py-1 pr-3">{action}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </section>

    <section>
        <h2 class="text-base font-bold m-0 mb-2.5">FAQ</h2>
        <dl class="text-sm m-0">
            <dt class="font-bold mt-3">What is FTR?</dt>
            <dd class="opacity-70 mt-1 leading-normal ml-0">
                First-Try Rate — the percentage of AI coding tasks completed
                without needing corrections. Higher is better.
            </dd>
            <dt class="font-bold mt-3">What is the daemon?</dt>
            <dd class="opacity-70 mt-1 leading-normal ml-0">
                A background process that indexes your repositories and watches
                for new sessions. It runs at <code
                    class="bg-surface-z3 px-1 py-px rounded-sm font-mono text-xs"
                    >localhost:7749</code
                > by default.
            </dd>
            <dt class="font-bold mt-3">What does memory scope mean?</dt>
            <dd class="opacity-70 mt-1 leading-normal ml-0">
                Memories can be <em>project-scoped</em> (apply to one project)
                or shared to the <em>collective</em> (apply globally across all projects).
            </dd>
        </dl>
    </section>
</div>

<style>
    .shortcuts-row {
        border-bottom: 1px solid oklch(var(--color-surface-z2) / 1);
    }
</style>
