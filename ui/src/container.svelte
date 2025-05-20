<script lang="ts">
    import type { ContainerId } from "./constants";

    import { onMount } from "svelte";
    import { appState, attachCanvas, detachCanvas } from "./protocol.svelte"; // @TEMP

    let {
        id,
        size,
        activeId = $bindable(),
        displayPaneOpen = $bindable(),
    }: {
        id: ContainerId;
        size: number;
        activeId: ContainerId;
        displayPaneOpen: boolean;
    } = $props();

    // @TEMP {
    let canvasState: {
        canvas: HTMLCanvasElement;
        deviceId: number;
        streamIndex: number;
        canvasId: number;
    } = $state({
        canvas: null,
        deviceId: null,
        streamIndex: 0,
        canvasId: null,
    });
    //const streamIndex = 0;
    $effect(() => {
        if (
            canvasState.deviceId == null &&
            appState.shared.devices.length > 0
        ) {
            canvasState.deviceId = appState.shared.devices[0].id;
        }
    });
    $effect(() => {
        if (
            canvasState.canvasId == null &&
            canvasState.deviceId != null &&
            canvasState.canvas != null
        ) {
            canvasState.canvasId = attachCanvas(
                canvasState.deviceId,
                canvasState.streamIndex,
                canvasState.canvas,
            );
        }
    });
    onMount(() => {
        return () => {
            if (canvasState.canvasId != null) {
                detachCanvas(
                    canvasState.deviceId,
                    canvasState.streamIndex,
                    canvasState.canvasId,
                );
            }
        };
    });
    // }
</script>

<div
    class="container {activeId === id ? 'active' : ''}"
    tabindex={id}
    role="button"
    onclick={() => {
        if (activeId === id) {
            activeId = 0;
        } else {
            activeId = id;
            displayPaneOpen = true;
        }
    }}
    onkeydown={() => {}}
    style="flex-grow: {size}"
>
    <canvas bind:this={canvasState.canvas} class="painter"></canvas>
</div>

<style>
    .container {
        flex-grow: 1;
        flex-basis: 0;
        background-color: var(--background-1);
        border-radius: 8px;
        border: 1px solid var(--background-0);
        position: relative;
        overflow: hidden;
    }

    .container.active {
        border: 1px solid var(--blue-1);
    }

    .painter {
        position: absolute;
        width: 100%;
        height: 100%;
        object-position: center center;
        object-fit: contain;
    }
</style>
