<script lang="ts">
    let {
        open = $bindable(),
        onClose,
    }: {
        open: boolean;
        onClose?: () => void;
    } = $props();
</script>

<div
    class="mask {open ? '' : 'hidden'}"
    onclick={() => {
        if (onClose) {
            onClose();
        }
        open = false;
    }}
    role="none"
></div>

<style>
    .hidden {
        display: none;
    }

    .mask {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        z-index: 10;
    }
</style>
