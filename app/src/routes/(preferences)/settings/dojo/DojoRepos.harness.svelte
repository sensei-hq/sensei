<script lang="ts">
  import DojoRepos from './DojoRepos.svelte';
  import type { DojoGroup } from './dojo-connection.js';
  import type { ScreenNoticeProps } from './dojo-connection-state.svelte.js';

  let {
    dojos,
    ontoggle,
    electionNotice = null,
    electionsError = null,
  }: {
    dojos: DojoGroup[];
    ontoggle?: (key: string, elected: boolean) => void;
    electionNotice?: ScreenNoticeProps | null;
    electionsError?: string | null;
  } = $props();
</script>

<DojoRepos {dojos} {ontoggle} {electionNotice} {electionsError} />
