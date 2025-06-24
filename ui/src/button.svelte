<script lang="ts">
    import type { Snippet } from "svelte";

    let {
        label,
        icon,
        iconStyle,
        disabled,
        onClick = $bindable(),
    }: {
        label: string;
        icon?: Snippet<[]>;
        iconStyle?: string;
        disabled?: boolean;
        onClick: () => void;
    } = $props();
</script>

<div class="button-wrapper">
    <div
        class="button {disabled ? 'disabled' : ''}"
        onclick={() => {
            if (disabled == null || !disabled) {
                onClick();
            }
        }}
        role="none"
    >
        {#if icon != null}
            <div class="icon" style={iconStyle ?? ""}>
                {@render icon()}
            </div>
        {/if}
        <div class="label">{label}</div>
    </div>
</div>

<style>
    .button-wrapper {
        display: flex;
        align-self: center;
        padding-top: 10px;
    }

    .button {
        height: 34px;
        line-height: 34px;
        padding-left: 8px;
        padding-right: 8px;
        background-color: var(--button-background);
        cursor: pointer;
        border-radius: 6px;
        user-select: none;
        display: flex;
        align-items: center;
        gap: 5px;
    }

    .button.disabled {
        background-color: var(--button-background-disabled);
        cursor: default;
    }

    .icon {
        flex-grow: 0;
        flex-shrink: 0;
        width: 14px;
        height: 14px;
    }

    .icon :global(svg) {
        display: block;
    }

    .label {
        flex-grow: 0;
        flex-shrink: 0;
        font-size: 14px;
        color: var(--content-3);
    }

    .button:hover {
        background-color: var(--button-background-hover);
    }

    .button:active {
        background-color: var(--button-background-active);
    }

    .button.disabled:hover {
        background-color: var(--button-background-disabled);
    }

    .button.disabled:active {
        background-color: var(--button-background-disabled);
    }
</style>
