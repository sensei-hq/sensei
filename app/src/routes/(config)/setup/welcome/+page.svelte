<script lang="ts">
    const pillars = [
        { kanji: "観", title: "Observe", sub: "FTR · turns · corrections" },
        { kanji: "師", title: "Teach", sub: "patterns · rules · skills" },
        { kanji: "静", title: "Local", sub: "on your machine" },
    ];
</script>

<div class="max-w-[680px] mt-2.5">
    <div class="text-xs text-ink-soft tracking-wide uppercase mb-2">
        <span class="kanji text-accent mr-1.5 opacity-80">礼</span>· Welcome
    </div>
    <h1
        class="display text-4xl font-light leading-[1.08] m-0 mb-8 tracking-tight"
    >
        A teacher does not<br />
        <span class="text-accent">write the code.</span>
    </h1>

    <p class="text-base text-ink-mute leading-loose max-w-[560px] m-0 mb-7">
        Sensei watches how you and your AI assistants work together — the
        sessions that completed cleanly, the ones that didn't, and the patterns
        underneath both.
    </p>

    <p class="text-sm text-ink-soft leading-loose max-w-[560px] m-0 mb-11">
        The next few minutes: install the local components, point to your
        folders, confirm what was found. Nothing leaves your machine.
    </p>

    <div
        class="grid grid-cols-3 gap-4 py-6 border-t border-b border-paper-edge"
    >
        {#each pillars as pillar}
            <div>
                <div class="kanji text-2xl text-accent mb-2">
                    {pillar.kanji}
                </div>
                <div class="display text-xl">{pillar.title}</div>
                <div class="text-xs text-ink-soft mt-1">{pillar.sub}</div>
            </div>
        {/each}
    </div>
</div>
