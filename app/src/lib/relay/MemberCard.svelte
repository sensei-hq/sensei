<script lang="ts">
  // One crew member (COMPONENTS §3e MemberCard; CONTRACT §1h): full width — the role
  // mark in its presence tone, the codename (mono, in that tone; its title the status),
  // objective · code, the family and model tags, Message; not responding: the warning
  // border and its line under the card. On a phone: mark, name, Message; tags below.
  import { Pill } from '@rokkit/ui';
  import type { RelayMemberView, RelayPresence } from '@sensei/state';
  import RoleMark from './RoleMark.svelte';

  let { member, onmessage }: { member: RelayMemberView; onmessage: (laneId: string) => void } = $props();

  // Literal, so UnoCSS finds each.
  const TONE: Record<RelayPresence, string> = {
    active: 'text-success',
    idle: 'text-ink-mute',
    not_responding: 'text-warning',
  };
</script>

<article
  data-member={member.laneCode}
  data-status={member.status}
  class={[
    'flex flex-wrap items-center gap-x-3.5 gap-y-2 py-3 px-3.5 rounded-[10px] border border-solid bg-paper',
    member.status === 'not_responding' ? 'border-[color-mix(in_oklab,var(--warning)_55%,var(--paper))]' : 'border-paper-edge',
  ]}
>
  <RoleMark role={member.role} status={member.status} size={30} />
  <span class="flex-[1_1_calc(100%-120px)] md:flex-[0_0_240px] min-w-0 flex flex-col">
    <span data-member-codename title={member.statusText} class="font-mono text-sm font-semibold truncate {TONE[member.status]}">{member.codename}</span>
    <span data-member-lane class="text-xs text-ink-soft truncate">{member.lane}</span>
  </span>
  <span data-member-tags class="order-3 md:order-none basis-full md:basis-auto pl-11 md:pl-0 flex-[1_1_auto] min-w-0 flex flex-wrap gap-1.5">
    <Pill value={member.family} size="sm" />
    <Pill value={member.model} variant="outline" size="sm" class="font-mono" />
  </span>
  <button
    type="button"
    data-member-message
    aria-label="Message {member.codename}"
    title="Message {member.codename}"
    onclick={() => onmessage(member.id)}
    class="order-2 md:order-none ml-auto inline-flex items-center justify-center w-8 h-8 rounded-lg border border-solid border-paper-edge bg-paper text-ink cursor-pointer"
  ><span aria-hidden="true" class="i-solar-chat-round-line-linear text-base"></span></button>
  {#if member.alert}
    <p data-member-alert class="order-4 basis-full m-0 pl-11 text-xs text-warning">{member.alert}</p>
  {/if}
</article>
