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
        properties = $bindable(),
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

    const sigma = $derived.by(() => {
        const sigma =
            (parentWidth * wrapperHeight < wrapperWidth * parentHeight
                ? parentWidth / wrapperWidth
                : parentHeight / wrapperHeight) * properties.scale;
        return isNaN(sigma) ? 1.0 : sigma;
    });

    const width = $derived(wrapperWidth * sigma);
    const height = $derived(wrapperHeight * sigma);

    const control: {
        initialValue: [number, number];
        initialPosition: [number, number];
    } = $state({
        initialValue: null,
        initialPosition: null,
    });

    let wrapper: HTMLElement;
    let canvas: HTMLCanvasElement;
    let timestampOverlay: HTMLElement;
    onMount(() => {
        const elementId = attach(
            deviceId,
            streamIndex,
            canvas,
            timestampOverlay,
            properties,
        );
        properties.width = canvas.width;
        properties.height = canvas.height;
        wrapperWidth = canvas.width;
        wrapperHeight = canvas.height;
        return () => {
            detach(deviceId, streamIndex, elementId);
        };
    });

    const offsetX = $derived(
        parentWidth / 2 - sigma * (properties.targetX * properties.width),
    );
    const offsetY = $derived(
        parentHeight / 2 - sigma * (properties.targetY * properties.height),
    );
    const paintAreaLeft = $derived(Math.max(offsetX, 0));
    const paintAreaTop = $derived(Math.max(offsetY, 0));
    const paintAreaWidth = $derived(
        Math.min(width + Math.min(offsetX, 0), parentWidth),
    );
    const paintAreaHeight = $derived(
        Math.min(height + Math.min(offsetY, 0), parentHeight),
    );
    const reticleLinesCenters = $derived([
        Math.round(parentHeight / 2 - 5 - 0.5),
        Math.round(parentHeight / 2 + 5 - 0.5),
        Math.round(parentWidth / 2 - 5 - 0.5),
        Math.round(parentWidth / 2 + 5 - 0.5),
    ]);

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
            properties.targetX =
                (control.initialValue[0] - deltaX / sigma) / properties.width;
            properties.targetY =
                (control.initialValue[1] - deltaY / sigma) / properties.height;
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
        const previouScale = properties.scale;
        const previousSigma = sigma;
        const newScale = clamp(
            previouScale + event.deltaY * -0.01,
            1.0,
            constants.MAXIMUM_SCALE,
        );
        const newSigma = (newScale / previouScale) * previousSigma;
        const rectangle = wrapper.getBoundingClientRect();
        properties.scale = newScale;
        properties.targetX =
            ((event.clientX - rectangle.x) *
                ((newSigma - previousSigma) / (newSigma * previousSigma)) +
                (previousSigma / newSigma) *
                    (properties.targetX * properties.width)) /
            properties.width;
        properties.targetY =
            ((event.clientY - rectangle.y) *
                ((newSigma - previousSigma) / (newSigma * previousSigma)) +
                (previousSigma / newSigma) *
                    (properties.targetY * properties.height)) /
            properties.height;
    }}
    onmousedown={event => {
        control.initialValue = [
            $state.snapshot(properties.targetX * properties.width),
            $state.snapshot(properties.targetY * properties.height),
        ];
        control.initialPosition = [event.clientX, event.clientY];
    }}
    role="none"
>
    <canvas bind:this={canvas}></canvas>
</div>

<div class="reticle" style="display: {properties.reticle ? 'block' : 'none'};">
    <div
        class="reticle-line"
        style="width: {paintAreaWidth}px; height: 1px; left: {paintAreaLeft}px; top: {reticleLinesCenters[0]}px; visibility: {reticleLinesCenters[0] -
            0.5 >=
            paintAreaTop &&
        reticleLinesCenters[0] + 0.5 < paintAreaTop + paintAreaHeight
            ? 'visible'
            : 'hidden'}"
    ></div>
    <div
        class="reticle-line"
        style="width: {paintAreaWidth}px; height: 1px; left: {paintAreaLeft}px; top: {reticleLinesCenters[1]}px; visibility: {reticleLinesCenters[1] -
            0.5 >=
            paintAreaTop &&
        reticleLinesCenters[1] + 0.5 < paintAreaTop + paintAreaHeight
            ? 'visible'
            : 'hidden'}"
    ></div>
    <div
        class="reticle-line"
        style="width: 1px; height: {paintAreaHeight}px; left: {reticleLinesCenters[2]}px; top: {paintAreaTop}px; visibility: {reticleLinesCenters[2] -
            0.5 >=
            paintAreaLeft &&
        reticleLinesCenters[2] + 0.5 < paintAreaLeft + paintAreaWidth
            ? 'visible'
            : 'hidden'}"
    ></div>
    <div
        class="reticle-line"
        style="width: 1px; height: {paintAreaHeight}px; left: {reticleLinesCenters[3]}px; top: {paintAreaTop}px;  visibility: {reticleLinesCenters[3] -
            0.5 >=
            paintAreaLeft &&
        reticleLinesCenters[3] + 0.5 < paintAreaLeft + paintAreaWidth
            ? 'visible'
            : 'hidden'}"
    ></div>
</div>

<div
    bind:this={timestampOverlay}
    class="timestamp"
    style="display: {properties.timestamp ? 'block' : 'none'}"
></div>

<style>
    .event-display {
        position: absolute;
    }

    .event-display canvas {
        width: 100%;
        height: 100%;
        image-rendering: pixelated;
    }

    .reticle {
        position: absolute;
        z-index: 1;
        pointer-events: none;
        left: 0;
        top: 0;
        right: 0;
        bottom: 0;
    }

    .reticle-line {
        position: absolute;
        background-color: var(--content-2);
        opacity: 0.8;
    }

    .timestamp {
        position: absolute;
        z-index: 1;
        font-size: 16px;
        top: 12px;
        left: 16px;
        color: var(--content-1);
        font-family: "Roboto Mono", monospace;
        user-select: none;
        pointer-events: none;
    }
</style>
