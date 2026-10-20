<script lang="ts">
  import SignInPending from './SignInPending.svelte';

  let {
    pending,
    copy,
  }: {
    pending: { label: string; expectedLogin: string | null; url: string } | null;
    copy?: (text: string) => Promise<void>;
  } = $props();
</script>

<SignInPending {pending} {copy} />
