<script lang="ts">
    import type { EventDisplayProperties } from "./appState.svelte";

    import * as constants from "./constants";
    import * as colormaps from "./colormaps";
    import { onMount } from "svelte";
    import { attach, detach } from "./protocol.svelte";

    let {
        deviceId,
        streamIndex,
        parentWidth,
        parentHeight,
        onCancelClick,
        properties,
    }: {
        deviceId: number;
        streamIndex: number;
        parentWidth: number;
        parentHeight: number;
        onCancelClick: () => void;
        properties: EventDisplayProperties;
    } = $props();

    let wrapperWidth: number = $state(0.0);
    let wrapperHeight: number = $state(0.0);
    let wrapperTargetX: number = $state(0.0);
    let wrapperTargetY: number = $state(0.0);
    let scale: number = $state(1.0);

    const sigma = $derived.by(() => {
        const sigma =
            (parentWidth * wrapperHeight < wrapperWidth * parentHeight
                ? parentWidth / wrapperWidth
                : parentHeight / wrapperHeight) * scale;
        return isNaN(sigma) ? 1.0 : sigma;
    });
    const width = $derived(wrapperWidth * sigma);
    const height = $derived(wrapperHeight * sigma);
    const offsetX = $derived(parentWidth / 2 - sigma * wrapperTargetX);
    const offsetY = $derived(parentHeight / 2 - sigma * wrapperTargetY);

    const control: {
        initialValue: [number, number];
        initialPosition: [number, number];
    } = $state({
        initialValue: null,
        initialPosition: null,
    });

    let wrapper: HTMLElement;
    let canvas: HTMLCanvasElement;
    let overlay: HTMLElement;
    onMount(() => {
        const elementId = attach(
            deviceId,
            streamIndex,
            canvas,
            overlay,
            properties,
        );
        wrapperWidth = canvas.width;
        wrapperHeight = canvas.height;
        wrapperTargetX = canvas.width / 2;
        wrapperTargetY = canvas.height / 2;
        return () => {
            detach(deviceId, streamIndex, elementId);
        };
    });

    function clamp(value: number, minimum: number, maximum: number): number {
        return Math.min(Math.max(value, minimum), maximum);
    }
</script>

<svelte:window
    onmouseup={() => {
        control.initialValue = null;
        control.initialPosition = null;
    }}
    onmousemove={event => {
        if (control.initialValue != null) {
            const deltaX = event.clientX - control.initialPosition[0];
            const deltaY = event.clientY - control.initialPosition[1];
            if (Math.hypot(deltaX, deltaY) > constants.CLICK_MAXIMUM_DISTANCE) {
                onCancelClick();
            }
            wrapperTargetX = control.initialValue[0] - deltaX / sigma;
            wrapperTargetY = control.initialValue[1] - deltaY / sigma;
        }
    }}
/>

<div
    bind:this={wrapper}
    class="event-display"
    style="width: {width}px; height: {height}px; transform: translate({offsetX}px, {offsetY}px); cursor: {control.initialValue ==
    null
        ? 'default'
        : 'move'}"
    onwheel={event => {
        event.preventDefault();
        const previouScale = scale;
        const previousSigma = sigma;
        const newScale = clamp(
            previouScale + event.deltaY * -0.01,
            1.0,
            constants.MAXIMUM_SCALE,
        );
        const newSigma = (newScale / previouScale) * previousSigma;
        const rectangle = wrapper.getBoundingClientRect();
        scale = newScale;
        wrapperTargetX =
            (event.clientX - rectangle.x) *
                ((newSigma - previousSigma) / (newSigma * previousSigma)) +
            (previousSigma / newSigma) * wrapperTargetX;
        wrapperTargetY =
            (event.clientY - rectangle.y) *
                ((newSigma - previousSigma) / (newSigma * previousSigma)) +
            (previousSigma / newSigma) * wrapperTargetY;
    }}
    onmousedown={event => {
        control.initialValue = [
            $state.snapshot(wrapperTargetX),
            $state.snapshot(wrapperTargetY),
        ];
        control.initialPosition = [event.clientX, event.clientY];
    }}
    role="button"
    tabindex={0}
>
    <canvas bind:this={canvas}></canvas>
</div>

<div bind:this={overlay} class="timestamp">00:00:00.000000</div>

<style>
    .event-display {
        position: absolute;
    }

    .event-display canvas {
        width: 100%;
        height: 100%;
        image-rendering: pixelated;
    }

    .timestamp {
        position: absolute;
        z-index: 1;
        font-size: 16px;
        top: 12px;
        left: 16px;
        color: #bbbbbb;
        font-family: "Roboto Mono", monospace;
        user-select: none;
    }
</style>
