<script lang="ts">
    import type { ContainerId } from "./constants";

    const {
        open,
        activeContainerId,
    }: { open: boolean; activeContainerId: ContainerId } = $props();
</script>

<div class="displaypane {open ? 'open' : ''}">
    <div class="content">
        {#if activeContainerId === 0}
            <div class="message">
                Select a layout item to edit its content and style.
            </div>
        {:else}
            <div class="message">
                Active container: {activeContainerId}
            </div>
        {/if}
    </div>
</div>

<style>
    .displaypane {
        width: 0;
        border-top: 1px solid var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        transition: width 0.3s;
        overflow: hidden;
        position: relative;
    }

    .displaypane.open {
        width: var(--display-pane-width);
    }

    .content {
        width: var(--display-pane-width);
        height: calc(100vh - 1px - var(--status-bar-height));
        background-color: var(--background-2);
        position: absolute;
        left: 0;
        top: 0;
        padding: 20px;
    }

    .message {
        color: #aaaaaa;
    }
</style>
