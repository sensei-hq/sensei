<script lang="ts">
  const stats = [
    { kanji: '観', label: 'Observing', sub: 'sessions tracked in real time' },
    { kanji: '師', label: 'Teaching', sub: 'after ~3 sessions per project' },
    { kanji: '静', label: 'Local', sub: 'nothing leaves your machine' },
  ];
</script>

<div class="max-w-[720px]">
  <h1 class="display text-[54px] font-light leading-[1.08] m-0 mb-8 tracking-[-0.02em]">
    The observatory<br/>
    <span class="text-primary-z5">is ready.</span>
  </h1>

  <p class="text-[16px] text-surface-z7 leading-[1.7] max-w-[560px] m-0 mb-11">
    Sensei is watching. Work normally with your AI assistants — sensei learns
    from every session and begins teaching after it has enough data.
  </p>

  <div class="grid grid-cols-3 gap-[18px] py-[22px] border-t border-b border-surface-z2">
    {#each stats as s}
      <div>
        <div class="kanji text-[26px] text-primary-z5 mb-2">{s.kanji}</div>
        <div class="display text-[20px]">{s.label}</div>
        <div class="text-xs text-surface-z6 mt-[3px]">{s.sub}</div>
      </div>
    {/each}
  </div>
</div>
