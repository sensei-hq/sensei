<script lang="ts">
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

<div class="flex flex-col items-center text-center py-20 gap-4">
    <span class="kanji text-4xl text-accent opacity-30">{kanji}</span>
    <p class="display text-xl font-normal m-0">{title}</p>
    <p class="text-sm text-ink-soft max-w-[380px] leading-normal m-0">
        {description}
    </p>
</div>
