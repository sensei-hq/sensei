<script lang="ts">
    const pillars = [
        { kanji: "観", title: "Observe", sub: "FTR · turns · corrections" },
        { kanji: "師", title: "Teach", sub: "patterns · rules · skills" },
        { kanji: "静", title: "Local", sub: "on your machine" },
    ];
</script>

<div class="max-w-[680px] mt-2.5">
    <div class="text-2xs text-surface-z6 tracking-cap uppercase mb-2">
        <span class="kanji text-primary-z6 mr-1.5 opacity-80">礼</span>· Welcome
    </div>
    <h1
        class="display text-[54px] font-light leading-[1.08] m-0 mb-8 tracking-tight"
    >
        A teacher does not<br />
        <span class="text-primary-z6">write the code.</span>
    </h1>

    <p class="text-base text-surface-z8 leading-reading max-w-[560px] m-0 mb-7">
        Sensei watches how you and your AI assistants work together — the
        sessions that completed cleanly, the ones that didn't, and the patterns
        underneath both.
    </p>

    <p class="text-sm text-surface-z6 leading-reading max-w-[560px] m-0 mb-11">
        The next few minutes: install the local components, point to your
        folders, confirm what was found. Nothing leaves your machine.
    </p>

    <div
        class="grid grid-cols-3 gap-4.5 py-5.5 border-t border-b border-surface-z2"
    >
        {#each pillars as pillar}
            <div>
                <div class="kanji text-[26px] text-primary-z6 mb-2">
                    {pillar.kanji}
                </div>
                <div class="display text-xl">{pillar.title}</div>
                <div class="text-xs text-surface-z6 mt-0.75">{pillar.sub}</div>
            </div>
        {/each}
    </div>
</div>
