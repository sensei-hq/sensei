<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
    let {
        kanji,
        title,
        description,
    }: {
        kanji: string;
        title: string;
        description: string;
    } = $props();
</script>

<div class="flex flex-col items-center text-center py-20 gap-4" data-empty>
    <Kanji char={kanji} size="inherit" tone="inherit" class="text-4xl text-accent opacity-30" />
    <p class="display text-xl font-normal m-0">{title}</p>
    <p class="text-sm text-ink-mute max-w-[380px] leading-normal m-0">
        {description}
    </p>
</div>
