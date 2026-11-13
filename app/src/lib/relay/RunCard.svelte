<script lang="ts">
  // The run, or the crew before its first run (COMPONENTS §3e RunCard; CONTRACT §1h): a
  // header — the title with its state, the meta line (it completes, and is archived, when
  // its last task is done), the facts — over the members by group. No End button.
  import type { RelayMemberGroupView, RelayRunCardView } from '@sensei/state';
  import MemberCard from './MemberCard.svelte';
  import Pill from './Pill.svelte';

  let {
    card,
    groups,
    onmessage,
  }: { card: RelayRunCardView; groups: RelayMemberGroupView[]; onmessage: (laneId: string) => void } = $props();
</script>

<section data-run-card aria-label={card.title} class="flex flex-col border border-solid border-paper-edge rounded-xl bg-paper overflow-hidden">
  <header class="flex flex-wrap items-start gap-x-6 gap-y-3 py-3.5 px-4 bg-paper-soft border-b border-solid border-paper-edge">
    <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-1">
      <h3 data-run-title class="m-0 flex flex-wrap items-center gap-2 display text-lg font-medium">
        <span>{card.title}</span>
        <Pill label={card.state.label} tone={card.state.tone} />
      </h3>
      <p data-run-meta class="m-0 text-xs text-ink-mute">{card.meta}</p>
    </div>
    <dl class="m-0 flex gap-4.5">
      {#each card.facts as fact (fact.label)}
        <div data-run-fact class="flex flex-col items-end">
          <dt class="text-xs uppercase tracking-[0.12em] text-ink-mute">{fact.label}</dt>
          <dd class="m-0 display text-lg tabular-nums">{fact.value}</dd>
        </div>
      {/each}
    </dl>
  </header>
  {#each groups as group (group.label)}
    <div data-member-group class="flex flex-col gap-2 pt-3 px-4 pb-3.5">
      <p data-member-group-label class="m-0 text-xs uppercase tracking-[0.12em] text-ink-mute">{group.label}</p>
      {#each group.members as member (member.id)}
        <MemberCard {member} {onmessage} />
      {/each}
    </div>
  {/each}
</section>
