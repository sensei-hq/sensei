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
    <span class="kanji text-6xl text-primary-z5 opacity-30">{kanji}</span>
    <p class="display text-xl font-normal m-0">{title}</p>
    <p class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0">
        {description}
    </p>
</div>
