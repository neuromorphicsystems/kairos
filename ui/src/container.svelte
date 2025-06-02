<script lang="ts">
    import type { ContainerId } from "./constants";

    import * as utilities from "./utilities";
    import appState from "./appState.svelte";
    import EventDisplay from "./eventDisplay.svelte";
    import SampleDisplay from "./sampleDisplay.svelte";

    let {
        id,
        size,
        activeId = $bindable(),
    }: {
        id: ContainerId;
        size: number;
        activeId: ContainerId;
    } = $props();

    let width: number = $state(0);
    let height: number = $state(0);
    let holdingMouseButton: boolean = $state(false);

    const display = $derived(appState.local.displays[id - 1]);
</script>

<svelte:window
    onmouseup={() => {
        holdingMouseButton = false;
    }}
/>

<div
    bind:clientWidth={width}
    bind:clientHeight={height}
    class="container {activeId === id ? 'active' : ''}"
    style="flex-grow: {size}"
    role="button"
    tabindex={id}
    onmousedown={() => {
        holdingMouseButton = true;
    }}
    onmouseup={() => {
        if (holdingMouseButton) {
            if (activeId === id) {
                activeId = 0;
            } else {
                activeId = id;
            }
        }
        holdingMouseButton = false;
    }}
>
    {#if display != null && display.target != null}
        {#if display.properties.type === "EventDisplayProperties"}
            <EventDisplay
                deviceId={display.target.deviceId}
                streamIndex={display.target.streamIndex}
                parentWidth={width}
                parentHeight={height}
                onCancelClick={() => {
                    holdingMouseButton = false;
                }}
                properties={display.properties}
            ></EventDisplay>
        {:else if display.properties.type === "SampleDisplayProperties"}
            <SampleDisplay
                deviceId={display.target.deviceId}
                streamIndex={display.target.streamIndex}
                parentWidth={width}
                parentHeight={height}
                orientation={display.properties.orientation}
                onCancelClick={() => {
                    holdingMouseButton = false;
                }}
                properties={display.properties}
            ></SampleDisplay>
        {/if}
    {/if}
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
</style>
