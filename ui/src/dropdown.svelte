<script lang="ts">
    import type { Snippet } from "svelte";

    import PopoverMask from "./popoverMask.svelte";

    let {
        choices,
        iconStyle,
        selectedIndex = $bindable(),
        onChange,
    }: {
        choices: [string, Snippet<[string]> | null][];
        iconStyle?: string;
        selectedIndex: number;
        onChange?: (number) => void;
    } = $props();

    let open: boolean = $state(false);
    let popover: HTMLDivElement;
</script>

<PopoverMask bind:open></PopoverMask>

<div
    class="dropdown"
    onclick={event => {
        const buttonBoundingClientRect =
            event.currentTarget.getBoundingClientRect();
        popover.style.minWidth = `${buttonBoundingClientRect.width}px`;
        const popoverBoundingClientRect = popover.getBoundingClientRect();
        const leftAnchor =
            buttonBoundingClientRect.x + popoverBoundingClientRect.width <
            window.innerWidth - 8;
        const bottomAnchor =
            buttonBoundingClientRect.y +
                buttonBoundingClientRect.height +
                4 +
                popoverBoundingClientRect.height <
            window.innerHeight - 8;
        if (leftAnchor) {
            popover.style.left = `${buttonBoundingClientRect.x}px`;
        } else {
            popover.style.left = `${buttonBoundingClientRect.x + buttonBoundingClientRect.width - popoverBoundingClientRect.width}px`;
        }
        if (bottomAnchor) {
            popover.style.top = `${buttonBoundingClientRect.y + buttonBoundingClientRect.height + 4}px`;
        } else {
            popover.style.top = `${buttonBoundingClientRect.y - popoverBoundingClientRect.height - 4}px`;
        }
        open = true;
    }}
    role="none"
>
    <div class="value">
        {#if choices[selectedIndex][1] != null}
            <div class="icon" style={iconStyle ?? ""}>
                {@render choices[selectedIndex][1](choices[selectedIndex][0])}
            </div>
        {/if}
        <div class="label">
            {choices[selectedIndex][0]}
        </div>
    </div>
    <div class="arrow">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
            ><polyline
                fill="none"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="10"
                points="25 12 25 62 75 62"
                transform="rotate(-45 50 37)"
            /></svg
        >
    </div>
</div>

<div class="popover {open ? '' : 'hidden'}" bind:this={popover}>
    {#each choices as choice, index}
        <div
            class="choice"
            onclick={() => {
                if (index !== selectedIndex) {
                    if (onChange) {
                        onChange(index);
                    }
                    selectedIndex = index;
                }
                open = false;
            }}
            role="none"
        >
            <div class="tick">
                {#if index === selectedIndex}
                    &#10003;
                {/if}
            </div>
            {#if choice[1] != null}
                <div class="icon" style={iconStyle ?? ""}>
                    {@render choice[1](choice[0])}
                </div>
            {/if}
            <div class="label">{choice[0]}</div>
        </div>
    {/each}
</div>

<style>
    .dropdown {
        width: 100%;
        height: 34px;
        padding-top: 4px;
        padding-bottom: 4px;
        padding-left: 8px;
        padding-right: 8px;
        background-color: var(--button-background);
        cursor: pointer;
        border-radius: 6px;
        display: flex;
        justify-content: space-between;
        font-size: 14px;
        color: var(--content-3);
        align-items: center;
        user-select: none;
        position: relative;
    }

    .dropdown:hover {
        background-color: var(--button-background-hover);
    }

    .dropdown .value {
        flex-shrink: 1;
        flex-grow: 1;
        display: flex;
        align-items: center;
        overflow: hidden;
    }

    .dropdown .label {
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    .dropdown .arrow {
        flex-shrink: 0;
        flex-grow: 0;
        padding-right: 4px;
        padding-left: 8px;
    }

    .dropdown .arrow svg {
        width: 16px;
        height: 16px;
    }

    .dropdown .arrow svg polyline {
        stroke: var(--content-3);
    }

    .popover {
        position: fixed;
        left: 0;
        top: 0;
        background-color: var(--background-3);
        border: 1px solid var(--border);
        border-radius: 8px;
        z-index: 11;
        overflow: hidden;
        font-size: 14px;
        color: var(--content-3);
        box-shadow: 0 0 8px 0 #00000080;
    }

    .choice {
        cursor: pointer;
        padding: 8px;
        display: flex;
        align-items: center;
        user-select: none;
    }

    .choice:hover {
        background-color: var(--button-background-hover);
    }

    .tick {
        flex-shrink: 0;
        flex-grow: 0;
        width: 20px;
        font-size: 14px;
    }

    .hidden {
        visibility: hidden;
    }
</style>
