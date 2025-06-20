<script lang="ts">
    import * as utilities from "./utilities";

    let {
        label,
        description,
        rightSide,
        value = $bindable(),
        units,
        step,
        min,
        max,
        labelWidth,
        labelBold,
        inputWidth,
        digits,
        paddingTop,
        onChange,
    }: {
        label: string;
        description: string;
        rightSide: boolean;
        value: number;
        units: string | null;
        step: number | "any";
        min: number | null;
        max: number | null;
        labelWidth: number;
        labelBold: boolean;
        inputWidth: number;
        digits: number;
        paddingTop: number;
        onChange?: (number) => void;
    } = $props();

    let showHelp: boolean = $state(false);
    let helpPopover: HTMLDivElement;
    let helpPopoverArrow: HTMLDivElement;

    const verticalOffset: number = 14.0;
</script>

<div class="number-input" style="padding-top: {paddingTop}px">
    <div
        class="label {labelBold ? 'bold' : ''}"
        style="width: {labelWidth}px; text-align: {rightSide
            ? 'left'
            : 'right'}; margin-right: {rightSide ? 0 : 10}px"
    >
        <span
            onmouseenter={event => {
                const labelBoundingClientRect =
                    event.currentTarget.getBoundingClientRect();
                const popoverBoundingClientRect =
                    helpPopover.getBoundingClientRect();
                const x =
                    (labelBoundingClientRect.left +
                        labelBoundingClientRect.right) /
                    2;
                if (rightSide) {
                    if (
                        x - 6 - 25 + popoverBoundingClientRect.width <
                        window.innerWidth - 20
                    ) {
                        helpPopover.style.left = `${x - 6 - 25}px`;
                    } else {
                        helpPopover.style.left = `${window.innerWidth - 20 - popoverBoundingClientRect.width}px`;
                    }
                } else {
                    if (x + 6 + 25 - popoverBoundingClientRect.width > 20) {
                        helpPopover.style.left = `${x + 6 + 25 - popoverBoundingClientRect.width}px`;
                    } else {
                        helpPopover.style.left = "20px";
                    }
                }
                helpPopoverArrow.style.left = `${utilities.clamp(x - 25, 20 + 6, window.innerWidth - 50 - 6)}px`;
                if (
                    labelBoundingClientRect.bottom +
                        verticalOffset +
                        popoverBoundingClientRect.height <
                    window.innerHeight - 8
                ) {
                    helpPopover.style.top = `${labelBoundingClientRect.bottom + verticalOffset}px`;
                    helpPopoverArrow.style.top = `${labelBoundingClientRect.bottom + verticalOffset - 14}px`;
                    helpPopoverArrow.style.transform = "none";
                } else {
                    helpPopover.style.top = `${labelBoundingClientRect.top - verticalOffset - popoverBoundingClientRect.height}px`;
                    helpPopoverArrow.style.top = `${labelBoundingClientRect.top - verticalOffset - 7}px`;
                    helpPopoverArrow.style.transform = "rotate(180deg)";
                }
                showHelp = true;
            }}
            onmouseleave={() => {
                showHelp = false;
            }}
            role="none">{@html label}</span
        >
    </div>
    <input
        type="number"
        style="width: {inputWidth}px"
        value={value.toFixed(digits)}
        onchange={event => {
            let newValue = parseFloat(event.currentTarget.value);
            if (step !== "any") {
                newValue = Math.round(newValue / step) * step;
            }
            if (min != null && newValue < min) {
                newValue = min;
            }
            if (max != null && newValue > max) {
                newValue = max;
            }
            if (onChange) {
                onChange(newValue);
            }
            if (value === newValue) {
                event.currentTarget.value = newValue.toString();
            } else {
                value = newValue;
            }
        }}
        {min}
        {max}
        {step}
    />
    {#if units != null}
        <div
            class="units"
            style="right: calc(var(--display-pane-width) - 40px - {labelWidth +
                inputWidth -
                8}px)"
        >
            {units}
        </div>
    {/if}

    <div
        class="help-popover {showHelp ? '' : 'hidden'}"
        bind:this={helpPopover}
    >
        <div class="description">
            {@html description}
        </div>
        <div class="arrow" bind:this={helpPopoverArrow}>
            <svg
                xmlns="http://www.w3.org/2000/svg"
                width="50"
                height="17"
                viewBox="0 0 50 17"
                ><path
                    fill="#333333"
                    fill-rule="evenodd"
                    stroke="#555555"
                    d="M-35,17 L-35,14.5 L9.58334938,14.5 C11.1339256,14.5 12.6242951,13.8997163 13.7420017,12.8249984 L23.6137826,3.33290138 C24.3880055,2.58845622 25.6119945,2.58845622 26.3862174,3.33290138 L36.2579983,12.8249984 C37.3757049,13.8997163 38.8660744,14.5 40.4166506,14.5 L80,14.5 L80,14.5 L80,17"
                /></svg
            >
        </div>
    </div>
</div>

<style>
    .number-input {
        display: flex;
        align-items: center;
        position: relative;
    }

    .label {
        font-size: 14px;
        color: var(--content-0);
    }

    .label.bold {
        font-weight: 700;
        color: var(--content-2);
    }

    .label span {
        cursor: default;
    }

    .label span:hover {
        color: var(--content-2);
    }

    .label.bold span:hover {
        color: var(--content-3);
    }

    input {
        height: 34px;
        padding-top: 4px;
        padding-bottom: 4px;
        padding-left: 8px;
        padding-right: 8px;
        background-color: var(--button-background);
        border-radius: 6px;
        font-size: 14px;
        color: var(--content-3);
        appearance: none;
        border: none;
        border: 1px solid var(--button-background);
    }

    input:focus,
    input:focus-visible {
        border: 1px solid var(--blue-1);
        outline: none;
    }

    input[type="number"]::-webkit-inner-spin-button {
        appearance: none;
    }

    input[type="number"] {
        appearance: textfield;
        -moz-appearance: textfield;
    }

    .units {
        position: absolute;
        font-size: 14px;
        color: var(--content-1);
    }

    .help-popover {
        pointer-events: none;
        position: fixed;
        background-color: var(--background-3);
        border: 1px solid var(--border);
        border-radius: 6px;
        z-index: 11;
        padding: 10px;
        box-shadow: 0 0 8px 0 #00000080;
        top: 0;
        left: 0;
    }

    .description {
        font-size: 14px;
        color: var(--content-3);
        max-width: 350px;
        text-align: justify;
    }

    .arrow {
        position: fixed;
        top: 0;
        left: 0;
        z-index: 12;
    }

    .hidden {
        visibility: hidden;
    }

    .description :global(i) {
        font-style: normal;
        margin-right: 10px;
        color: var(--content-0);
        font-size: 14px;
        font-weight: 400;
    }

    .description :global(code) {
        font-family: "RobotoMono", monospace;
        font-size: 12px;
    }

    .description :global(p) {
        margin: 0;
    }

    .description :global(p:not(:first-of-type)) {
        padding-top: 10px;
    }
</style>
