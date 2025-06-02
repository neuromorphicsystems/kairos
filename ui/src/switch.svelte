<script lang="ts">
    let {
        checked = $bindable(),
        onChange,
    }: {
        checked: boolean;
        onChange?: (boolean) => void;
    } = $props();
</script>

<input class="input" type="checkbox" bind:checked />

<style>
    .input {
        display: block;
    }
</style>
