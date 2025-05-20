<script lang="ts">
    const { open }: { open: boolean } = $props();
</script>

<style>
    .devicepane {
        width: 0;
        border-top: 1px solid var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        transition: width 0.3s;
        overflow: hidden;
        position: relative;
    }

    .devicepane.open {
        width: var(--device-pane-width);
    }

    .content {
        width: var(--device-pane-width);
        height: calc(100vh - 1px - var(--status-bar-height));
        background-color: var(--background-2);
        position: absolute;
        right: 0;
        top: 0;
    }
</style>

<div class="devicepane {open ? "open" : ""}">
    <div class=content>

    </div>
</div>
