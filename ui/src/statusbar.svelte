<script lang="ts">
    import type { ContainerId } from "./constants";

    import appState from "./appState.svelte";
    import LayoutPopover from "./layoutPopover.svelte";
    import PopoverMask from "./popoverMask.svelte";
    import RecordingsPopover from "./recordingsPopover.svelte";

    let {
        devicePaneOpen = $bindable(),
        displayPaneOpen = $bindable(),
        activeDisplayId = $bindable(),
    }: {
        devicePaneOpen: boolean;
        displayPaneOpen: boolean;
        activeDisplayId: ContainerId;
    } = $props();

    let activeMenu: number = $state(0);
    let recordingsLeft: number = $state(0);
    let workspaceLeft: number = $state(0);
    let layoutLeft: number = $state(0);
    let recording = $derived(
        Object.values(appState.deviceIdToRecordState).some(
            recordState => recordState.recording != null,
        ),
    );
</script>

<PopoverMask
    open={activeMenu === 2}
    onClose={() => {
        activeMenu = 0;
    }}
></PopoverMask>

<div
    class="status-bar {appState.local.connectionStatus} {recording
        ? 'recording'
        : ''}"
>
    <div class="left">
        <button
            class={devicePaneOpen ? "active" : ""}
            aria-label="Record"
            onclick={() => {
                devicePaneOpen = !devicePaneOpen;
            }}
        >
            <div class="icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
                    ><path
                        d="M36.7179487,18 L32.9230769,27.4871795 L19.6410256,27.4871795 C15.9618974,27.4871795 13,30.4490769 13,34.1282051 L13,72.0769421 C13,75.7560703 15.9618974,78.7179677 19.6410256,78.7179677 L80.3589744,78.7179677 C84.0381026,78.7179677 87,75.7560703 87,72.0769421 L87,34.1282051 C87,30.4490769 84.0381026,27.4871795 80.3589744,27.4871795 L67.0769231,27.4871795 L63.2820513,18 L36.7179487,18 L36.7179487,18 Z M50,35.0769136 C59.9215974,35.0769136 68.025641,43.1809762 68.025641,53.1025831 C68.025641,63.0240856 59.9215974,71.1282241 50,71.1282241 C40.0784974,71.1282241 31.974359,63.0241805 31.974359,53.1025831 C31.974359,43.1809667 40.0784974,35.0769136 50,35.0769136 L50,35.0769136 Z M50,40.7692308 C43.155,40.7692308 37.6666667,46.2573933 37.6666667,53.1025831 C37.6666667,59.9476779 43.155,65.4359164 50,65.4359164 C56.8452846,65.4359164 62.3333333,59.9477728 62.3333333,53.1025831 C62.3333333,46.2573933 56.8452846,40.7692308 50,40.7692308 L50,40.7692308 Z"
                    /></svg
                >
            </div>
            <div class="label">Record</div>
        </button>

        <div class="menu">
            <button
                class={activeMenu === 1 ? "active" : ""}
                aria-label="Recordings"
                onclick={event => {
                    recordingsLeft =
                        event.currentTarget.getBoundingClientRect().left;
                    activeMenu = 1;
                }}
            >
                <div class="label">Recordings</div>
            </button>
            <button
                class={activeMenu === 2 ? "active" : ""}
                aria-label="Workspace"
                onclick={event => {
                    workspaceLeft =
                        event.currentTarget.getBoundingClientRect().left;
                    activeMenu = 2;
                }}
            >
                <div class="label">Workspace</div>
            </button>
            <button
                class={activeMenu === 3 ? "active" : ""}
                aria-label="Layout"
                onclick={event => {
                    layoutLeft =
                        event.currentTarget.getBoundingClientRect().left;
                    activeMenu = 3;
                }}
            >
                <div class="label">Layout</div>
            </button>
        </div>
    </div>
    <div class="center">
        {#if appState.local.connectionStatus === "connecting"}
            <div class="connection-label">Connecting...</div>
        {:else if appState.local.connectionStatus === "disconnected"}
            <div class="connection-label">Disconnected</div>
        {:else if recording}
            <div class="connection-label">Recording</div>
        {/if}
    </div>
    <div class="right">
        <button
            class={displayPaneOpen ? "active" : ""}
            aria-label="Apperance"
            onclick={() => {
                displayPaneOpen = !displayPaneOpen;
            }}
        >
            <div class="icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
                    ><path
                        d="M70.2601433,41.4372592 L70.2601433,19.1215627 C70.2601433,17.9429168 69.3172266,17 68.1385806,17 L59.0237187,17 C58.1593783,17 57.2950379,17.5500348 57.0593087,18.4143751 L53.8376765,28.1578483 L50.5374679,18.4143751 C50.2231623,17.5500348 49.4373983,17 48.573058,17 L40.3225364,17 C39.458196,17 38.7510084,17.4714584 38.4367028,18.2572223 L36.3151401,23.0503825 L34.1935774,18.2572223 C33.8792719,17.4714584 33.0935079,17 32.3077439,17 L25.0001391,17 C23.8214932,17 22.8785764,17.9429168 22.8785764,19.1215627 L22.8785764,41.4372592 L70.2601433,41.4372592 L70.2601433,41.4372592 Z M22.8,46.3089958 L22.8,51.4164615 C22.8,55.5810105 25.6287503,59.1955248 29.7147229,60.2170179 L38.9867376,62.4957334 C39.9296544,62.7314626 40.4796891,63.595803 40.4011128,64.5387197 L38.2009737,81.9826797 C37.8866681,84.4185479 38.672432,86.8544162 40.24396,88.6616733 C41.8154879,90.4689305 44.1727798,91.569 46.6086481,91.569 C49.0445163,91.569 51.3232318,90.5475068 52.9733361,88.6616733 C54.5448641,86.8544162 55.330628,84.4185479 55.0163224,81.9826797 L52.8161834,64.5387197 C52.6590306,63.595803 53.2876417,62.7314626 54.2305585,62.4957334 L63.5025732,60.2170179 C67.5885458,59.1955248 70.4172961,55.5810105 70.4172961,51.4164615 L70.4172961,46.3089958 L22.8,46.3089958 Z M46.5300717,86.2258051 C44.8013909,86.2258051 43.3870158,84.8114299 43.3870158,83.0827492 C43.3870158,81.3540685 44.8013909,79.9396934 46.5300717,79.9396934 C48.2587524,79.9396934 49.6731275,81.3540685 49.6731275,83.0827492 C49.6731275,84.8114299 48.2587524,86.2258051 46.5300717,86.2258051 Z"
                        transform="rotate(45 46.609 54.284)"
                    /></svg
                >
            </div>
            <div class="label">Display</div>
        </button>
    </div>
</div>

<RecordingsPopover
    open={activeMenu === 1}
    left={recordingsLeft}
    onClose={() => {
        activeMenu = 0;
    }}
></RecordingsPopover>

<div
    class="popover {activeMenu === 2 ? '' : 'hidden'}"
    style="left: {workspaceLeft}px"
>
    <div class="choice" onclick={() => {}} role="none">
        <div class="label">Load workspace...</div>
    </div>
    <div class="choice" onclick={() => {}} role="none">
        <div class="label">Save workspace...</div>
    </div>
</div>

<LayoutPopover
    open={activeMenu === 3}
    bind:activeDisplayId
    left={layoutLeft}
    onClose={() => {
        activeMenu = 0;
    }}
></LayoutPopover>

<style>
    .status-bar {
        display: flex;
        align-items: center;
        height: var(--status-bar-height);
    }

    .status-bar.connected {
        background-color: var(--background-3);
    }

    .status-bar.connecting {
        background-color: color-mix(
            in oklab,
            var(--background-3) 30%,
            var(--yellow-1) 70%
        );
    }

    .status-bar.disconnected {
        background-color: color-mix(
            in oklab,
            var(--background-3) 30%,
            var(--red-1) 70%
        );
    }

    .status-bar.connected.recording {
        background-color: color-mix(
            in oklab,
            var(--background-3) 30%,
            var(--blue-1) 70%
        );
    }

    .left {
        flex-grow: 0;
        flex-shrink: 0;
        padding-left: 8px;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .menu {
        border-left: 1px solid var(--border);
        padding-left: 8px;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .center {
        flex-shrink: 1;
        flex-grow: 1;
        display: flex;
        justify-content: space-around;
        align-items: center;
        font-size: 14px;
        color: var(--content-3);
    }

    .right {
        flex-grow: 0;
        flex-shrink: 0;
        padding-right: 8px;
        display: flex;
        justify-content: flex-end;
    }

    button {
        background: none;
        color: inherit;
        border: none;
        font: inherit;
        outline: inherit;
        display: flex;
        flex-direction: row;
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

    .icon {
        width: 26px;
        height: 26px;
    }

    .icon svg path {
        fill: var(--content-2);
    }

    button:hover .icon svg path {
        fill: var(--content-3);
    }

    .label {
        color: var(--content-2);
        font-size: 14px;
        height: 26px;
        line-height: 26px;
        padding-left: 4px;
        padding-right: 4px;
    }

    button:hover .label {
        color: var(--content-3);
    }

    .connection-label {
        color: var(--content-3);
    }

    .popover {
        position: fixed;
        background-color: var(--background-3);
        border: 1px solid var(--border);
        border-radius: 8px;
        z-index: 11;
        overflow: hidden;
        font-size: 14px;
        color: var(--content-3);
        box-shadow: 0 0 8px 0 #00000080;
        top: calc(var(--status-bar-height) - 4px);
        display: flex;
        flex-direction: column;
        padding: 10px;
        gap: 10px;
    }

    .choice {
        cursor: pointer;
        padding: 8px;
        display: flex;
        align-items: center;
        user-select: none;
        border-radius: 6px;
    }

    .choice:hover {
        background-color: var(--button-background-hover);
    }

    .hidden {
        visibility: hidden;
    }
</style>
