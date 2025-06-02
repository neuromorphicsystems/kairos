<script lang="ts">
    import type { SampleDisplayProperties } from "./appState.svelte";

    import * as constants from "./constants";
    import { onMount } from "svelte";
    import { attach, detach } from "./protocol.svelte";

    let {
        deviceId,
        streamIndex,
        parentWidth,
        parentHeight,
        orientation,
        onCancelClick,
        properties,
    }: {
        deviceId: number;
        streamIndex: number;
        parentWidth: number;
        parentHeight: number;
        orientation: "Auto" | "Row" | "Column";
        onCancelClick: () => void;
        properties: SampleDisplayProperties;
    } = $props();

    const control: {
        initialValue: [number, number];
        initialPosition: [number, number];
    } = $state({
        initialValue: null,
        initialPosition: null,
    });

    const actualOrientation = $derived.by(() => {
        switch (orientation) {
            case "Row": {
                return "row";
            }
            case "Column": {
                return "column";
            }
            default: {
                if (
                    parentWidth >
                    constants.CHART_AUTO_ORIENTATION_RATIO * parentHeight
                ) {
                    return "row";
                } else {
                    return "column";
                }
            }
        }
    });

    let element: HTMLElement;
    onMount(() => {
        const elementId = attach(
            deviceId,
            streamIndex,
            element,
            null,
            properties,
        );
        return () => {
            detach(deviceId, streamIndex, elementId);
        };
    });
</script>

<div
    class="sample-display"
    bind:this={element}
    style="flex-direction: {actualOrientation}"
></div>

<style>
    .sample-display {
        height: 100%;
        display: flex;
        align-items: stretch;
        padding: 10px;
        gap: 20px;
    }
</style>
