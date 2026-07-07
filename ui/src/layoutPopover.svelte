<script lang="ts">
    import type { ContainerId, Layout } from "./constants";

    import appState from "./appState.svelte";
    import PopoverMask from "./popoverMask.svelte";

    let {
        open,
        activeDisplayId = $bindable(),
        left,
        onClose,
    }: {
        open: boolean;
        activeDisplayId: ContainerId;
        left: number;
        onClose: () => void;
    } = $props();

    function updateLayout(layout: Layout) {
        const previousLayout = appState.local.layout;
        appState.local.layout = layout;
        switch (layout) {
            case "full":
                if (activeDisplayId > 1) {
                    activeDisplayId = 0;
                }
                break;
            case "h":
            case "v": {
                if (activeDisplayId > 2) {
                    activeDisplayId = 0;
                }
                if (appState.local.displays[1] == null) {
                    appState.local.displays[1] = appState.local.displays[0];
                }
                break;
            }
            case "hv1":
            case "hv2":
            case "vh1":
            case "vh2": {
                if (activeDisplayId > 3) {
                    activeDisplayId = 0;
                }
                if (appState.local.displays[1] == null) {
                    // @ts-ignore
                    appState.local.displays[1] = $state.snapshot(
                        appState.local.displays[0],
                    );
                }
                if (appState.local.displays[2] == null) {
                    // @ts-ignore
                    appState.local.displays[2] = $state.snapshot(
                        appState.local.displays[
                            layout === "vh1" || layout === "hv1" ? 0 : 1
                        ],
                    );
                }
                break;
            }
            case "hv1v2":
            case "vh1h2": {
                if (appState.local.displays[1] == null) {
                    // @ts-ignore
                    appState.local.displays[1] = $state.snapshot(
                        appState.local.displays[0],
                    );
                }
                if (
                    appState.local.displays[2] == null &&
                    appState.local.displays[3] == null
                ) {
                    // @ts-ignore
                    appState.local.displays[2] = $state.snapshot(
                        appState.local.displays[1],
                    );
                    // @ts-ignore
                    appState.local.displays[3] = $state.snapshot(
                        appState.local.displays[0],
                    );
                } else if (appState.local.displays[2] == null) {
                    // @ts-ignore
                    appState.local.displays[2] = $state.snapshot(
                        appState.local.displays[1],
                    );
                } else if (appState.local.displays[3] == null) {
                    switch (previousLayout) {
                        case "full":
                        case "h":
                        case "v":
                        case "hv2":
                        case "vh2":
                        case "hv1v2":
                        case "vh1h2": {
                            // @ts-ignore
                            appState.local.displays[3] = $state.snapshot(
                                appState.local.displays[0],
                            );
                            break;
                        }
                        case "hv1":
                        case "vh1": {
                            // @ts-ignore
                            appState.local.displays[3] = $state.snapshot(
                                appState.local.displays[2],
                            );
                            // @ts-ignore
                            appState.local.displays[2] = $state.snapshot(
                                appState.local.displays[1],
                            );
                            if (activeDisplayId === 3) {
                                activeDisplayId = 4;
                            }
                            break;
                        }
                        default: {
                            throw new Error(`unsupported layout ${layout}`);
                        }
                    }
                }
                break;
            }
            default: {
                throw new Error(`unsupported layout ${layout}`);
            }
        }
        onClose();
    }
</script>

<PopoverMask bind:open {onClose}></PopoverMask>

<div class="content {open ? '' : 'hidden'}" style="left: {left}px">
    <button
        aria-label="Layout h"
        class={appState.local.layout === "h" ? "active" : ""}
        onclick={() => {
            updateLayout("h");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="106" height="36" x="12" y="12" rx="8" /><rect
                        width="106"
                        height="36"
                        x="12"
                        y="52"
                        rx="8"
                    /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout hv2"
        class={appState.local.layout === "hv2" ? "active" : ""}
        onclick={() => {
            updateLayout("hv2");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="106" height="36" x="12" y="12" rx="8" /><rect
                        width="51"
                        height="36"
                        x="12"
                        y="52"
                        rx="8"
                    /><rect width="51" height="36" x="67" y="52" rx="8" /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout hv1"
        class={appState.local.layout === "hv1" ? "active" : ""}
        onclick={() => {
            updateLayout("hv1");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="106" height="36" x="12" y="52" rx="8" /><rect
                        width="51"
                        height="36"
                        x="12"
                        y="12"
                        rx="8"
                    /><rect width="51" height="36" x="67" y="12" rx="8" /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout v"
        class={appState.local.layout === "v" ? "active" : ""}
        onclick={() => {
            updateLayout("v");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="51" height="76" x="12" y="12" rx="8" /><rect
                        width="51"
                        height="76"
                        x="67"
                        y="12"
                        rx="8"
                    /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout vh2"
        class={appState.local.layout === "vh2" ? "active" : ""}
        onclick={() => {
            updateLayout("vh2");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="51" height="76" x="12" y="12" rx="8" /><rect
                        width="51"
                        height="36"
                        x="67"
                        y="52"
                        rx="8"
                    /><rect width="51" height="36" x="67" y="12" rx="8" /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout vh1"
        class={appState.local.layout === "vh1" ? "active" : ""}
        onclick={() => {
            updateLayout("vh1");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="51" height="76" x="67" y="12" rx="8" /><rect
                        width="51"
                        height="36"
                        x="12"
                        y="52"
                        rx="8"
                    /><rect width="51" height="36" x="12" y="12" rx="8" /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout full"
        class={appState.local.layout === "full" ? "active" : ""}
        onclick={() => {
            updateLayout("full");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><rect
                    width="106"
                    height="76"
                    x="12"
                    y="12"
                    fill-rule="evenodd"
                    rx="8"
                /></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout hv1v2"
        class={appState.local.layout === "hv1v2" ? "active" : ""}
        onclick={() => {
            updateLayout("hv1v2");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="66" height="36" x="12" y="12" rx="8" /><rect
                        width="36"
                        height="36"
                        x="12"
                        y="52"
                        rx="8"
                    /><rect width="66" height="36" x="52" y="52" rx="8" /><rect
                        width="36"
                        height="36"
                        x="82"
                        y="12"
                        rx="8"
                    /></g
                ></svg
            >
        </div>
    </button>
    <button
        aria-label="Layout vh1h2"
        class={appState.local.layout === "vh1h2" ? "active" : ""}
        onclick={() => {
            updateLayout("vh1h2");
        }}
    >
        <div class="icon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 100"
                ><g fill="none" fill-rule="evenodd"
                    ><rect width="51" height="52" x="12" y="12" rx="8" /><rect
                        width="51"
                        height="20"
                        x="12"
                        y="68"
                        rx="8"
                    /><rect width="51" height="52" x="67" y="36" rx="8" /><rect
                        width="51"
                        height="20"
                        x="67"
                        y="12"
                        rx="8"
                    /></g
                ></svg
            >
        </div>
    </button>
</div>

<style>
    .content.hidden {
        display: none;
    }

    .content {
        position: fixed;
        top: calc(var(--status-bar-height) - 4px);
        width: 222px;
        height: 186px;
        background-color: var(--background-3);
        border: 1px solid var(--border);
        border-radius: 8px;
        z-index: 11;
        display: flex;
        flex-wrap: wrap;
        padding: 10px;
        gap: 10px;
        box-shadow: 0 0 8px 0 #00000080;
    }

    button {
        background: none;
        color: inherit;
        border: none;
        font: inherit;
        outline: inherit;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 4px;
        border-radius: 6px;
        background-color: var(--background-3);
        cursor: pointer;
    }

    button:hover {
        background-color: var(--button-background-hover);
    }

    button.active {
        background-color: var(--button-background);
    }

    button.active:hover {
        background-color: var(--button-background-hover);
    }

    button .icon {
        width: 52px;
        height: 40px;
    }

    button .icon svg rect {
        fill: var(--content-2);
    }

    button:hover .icon svg rect {
        fill: var(--content-3);
    }
</style>
