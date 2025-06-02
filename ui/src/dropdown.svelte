<script lang="ts">
    let {
        choices,
        selectedIndex = $bindable(),
        onChange,
    }: {
        choices: string[];
        selectedIndex: number;
        onChange?: (number) => void;
    } = $props();
</script>

<select
    class="select"
    bind:value={selectedIndex}
    onchange={() => {
        if (onChange) {
            onChange(selectedIndex);
        }
    }}
>
    {#each choices as choice, index}
        <option value={index}>{choice}</option>
    {/each}
</select>

<style>
    .select {
        display: block;
    }
</style>
