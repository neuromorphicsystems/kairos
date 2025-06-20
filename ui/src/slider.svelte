<script lang="ts">
    let {
        value = $bindable(),
        step,
        min,
        max,
        onChange,
        logarithmic,
        ticks,
        width,
    }: {
        value: number;
        step: number | "any";
        min: number;
        max: number;
        onChange?: (number) => void;
        logarithmic: boolean;
        ticks: number[];
        width: number;
    } = $props();

    function valueToNormalizedPosition(
        value: number,
        min: number,
        max: number,
        logarithmic: boolean,
    ): number {
        return logarithmic
            ? Math.log(value / min) / Math.log(max / min)
            : (value - min) / (max - min);
    }

    function normalizedPositionToValue(
        normalizedPosition: number,
        min: number,
        max: number,
        logarithmic: boolean,
    ): number {
        return logarithmic
            ? Math.exp(normalizedPosition * Math.log(max / min)) * min
            : (max - min) * normalizedPosition + min;
    }

    const handleShadowSize = 22;
    const handleSize = 16;
    const ticksMagnetDistance = 0.02;
    const barWidth = $derived(width - handleShadowSize);
    const normalizedPosition = $derived(
        valueToNormalizedPosition(value, min, max, logarithmic),
    );
    const ticksNormalizedPositions = $derived(
        ticks.map(tick =>
            valueToNormalizedPosition(tick, min, max, logarithmic),
        ),
    );

    let control: {
        initialNormalizedPosition: number | null;
        initialPosition: number;
    } = $state({
        initialNormalizedPosition: null,
        initialPosition: 0.0,
    });

    function mouseDown(event: MouseEvent, isUpper: boolean) {
        control.initialNormalizedPosition = normalizedPosition;
        control.initialPosition = event.clientX;
    }

    function onNewNormalizedPosition(newNormalizedPosition: number) {
        for (const tickNormalizedPosition of ticksNormalizedPositions) {
            if (
                Math.abs(newNormalizedPosition - tickNormalizedPosition) <
                ticksMagnetDistance
            ) {
                newNormalizedPosition = tickNormalizedPosition;
                break;
            }
        }
        let newValue = normalizedPositionToValue(
            newNormalizedPosition,
            min,
            max,
            logarithmic,
        );
        if (step !== "any") {
            newValue = Math.round(newValue / step) * step;
        }
        if (newValue > max) {
            newValue = max;
        }
        if (newValue < min) {
            newValue = min;
        }
        if (newValue !== value) {
            if (onChange) {
                onChange(newValue);
            }
            value = newValue;
        }
    }

    function mouseMove(event: MouseEvent) {
        if (control.initialNormalizedPosition != null) {
            onNewNormalizedPosition(
                control.initialNormalizedPosition +
                    (event.clientX - control.initialPosition) / barWidth,
            );
        }
    }

    function mouseUp() {
        control.initialNormalizedPosition = null;
    }

    let bar: HTMLDivElement;
</script>

<svelte:window onmouseup={mouseUp} onmousemove={mouseMove} />

<div
    class="wrapper"
    style="width: {width}px; padding-left: {handleShadowSize /
        2}px; padding-right: {handleShadowSize / 2}px"
>
    <div class="slider" style="height: {handleShadowSize}px">
        <div
            bind:this={bar}
            class="bar"
            style="top: {(handleShadowSize - 4) / 2}px"
            onclick={event => {
                onNewNormalizedPosition(
                    (event.clientX - bar.getBoundingClientRect().x) / barWidth,
                );
            }}
            role="none"
        ></div>
        <div
            class="active-bar"
            style="left: 0; right: {(1.0 - normalizedPosition) *
                barWidth}px; top: {(handleShadowSize - 4) / 2}px"
            onclick={event => {
                onNewNormalizedPosition(
                    (event.clientX - bar.getBoundingClientRect().x) / barWidth,
                );
            }}
            role="none"
        ></div>
        <div
            class="handle-wrapper"
            style="left: {normalizedPosition * barWidth -
                handleSize /
                    2}px; width: {handleSize}px; height: {handleSize}px; top: {(handleShadowSize -
                handleSize) /
                2}px; z-index: 3;"
            onmousedown={event => mouseDown(event, true)}
            role="none"
        >
            <div
                class="shadow {control.initialNormalizedPosition == null
                    ? ''
                    : 'dragging'}"
                style="width: {handleShadowSize}px; height: {handleShadowSize}px; top: {(handleSize -
                    handleShadowSize) /
                    2}px; left: {(handleSize - handleShadowSize) /
                    2}px; border-radius: {handleShadowSize / 2}px;"
            ></div>
            <div
                class="handle"
                style="width: {handleSize}px; height: {handleSize}px; border-radius: {handleSize /
                    2}px;"
            ></div>
        </div>
    </div>
    {#if ticks.length > 0}
        <div class="ticks">
            {#each ticks as tick, index}
                <div
                    class="tick"
                    style="left: {ticksNormalizedPositions[index] * barWidth}px"
                    onclick={() => {
                        if (onChange) {
                            onChange(tick);
                        }
                        value = tick;
                    }}
                    role="none"
                >
                    <div class="marker"></div>
                    <div class="value">
                        {tick}
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>

<style>
    .wrapper {
        user-select: none;
    }

    .slider {
        position: relative;
    }

    .bar {
        height: 4px;
        border-radius: 2px;
        background-color: var(--background-1);
        position: absolute;
        left: 0;
        right: 0;
        z-index: 1;
        cursor: pointer;
    }

    .active-bar {
        height: 4px;
        border-radius: 2px;
        background-color: var(--content-1);
        position: absolute;
        z-index: 2;
        cursor: pointer;
    }

    .handle-wrapper {
        position: absolute;
    }

    .shadow {
        position: absolute;
        background-color: var(--content-1);
        opacity: 0;
        transition: opacity 0.2s;
    }

    .handle-wrapper:hover .shadow,
    .handle-wrapper:hover .shadow {
        opacity: 0.5;
    }

    .shadow.dragging {
        opacity: 0.5;
    }

    .handle {
        position: absolute;
        background-color: var(--content-1);
        cursor: pointer;
    }

    .ticks {
        position: relative;
        height: 30px;
    }

    .tick {
        position: absolute;
        display: flex;
        flex-direction: column;
        align-items: center;
        transform: translate(-50%);
        cursor: pointer;
    }

    .marker {
        width: 1px;
        height: 10px;
        background-color: var(--content-0);
    }

    .value {
        text-align: center;
        font-size: 12px;
        line-height: 20px;
        color: var(--content-0);
    }

    .tick:hover .marker {
        background-color: var(--content-2);
    }

    .tick:hover .value {
        color: var(--content-2);
    }
</style>
