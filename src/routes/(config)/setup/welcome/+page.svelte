<script lang="ts">
  const pillars = [
    { kanji: '観', title: 'Observe', sub: 'FTR · turns · corrections' },
    { kanji: '師', title: 'Teach',   sub: 'patterns · rules · skills' },
    { kanji: '静', title: 'Local',   sub: 'on your machine' },
  ];
</script>

<div class="max-w-[720px]">
  <h1 class="display text-[54px] font-light leading-[1.08] m-0 mb-8 tracking-[-0.02em]">
    A teacher does not<br/>
    <span class="text-primary-z5">write the code.</span>
  </h1>

  <p class="text-[16px] text-surface-z7 leading-[1.7] max-w-[560px] m-0 mb-7">
    Sensei watches how you and your AI assistants work together — the sessions that
    completed cleanly, the ones that didn't, and the patterns underneath both.
  </p>

  <p class="text-sm text-surface-z6 leading-[1.7] max-w-[560px] m-0 mb-11">
    The next few minutes: install the local components, point to your folders, confirm
    what was found. Nothing leaves your machine.
  </p>

  <div class="grid grid-cols-3 gap-[18px] py-[22px] border-t border-b border-surface-z2">
    {#each pillars as pillar}
      <div>
        <div class="kanji text-[26px] text-primary-z5 mb-2">{pillar.kanji}</div>
        <div class="display text-[20px]">{pillar.title}</div>
        <div class="text-xs text-surface-z6 mt-[3px]">{pillar.sub}</div>
      </div>
    {/each}
  </div>
</div>
