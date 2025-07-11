<script lang="ts">
    import type { ContainerId, Layout } from "./constants";

    import appState from "./appState.svelte";
    import Container from "./container.svelte";

    let {
        activeDisplayId = $bindable(),
    }: {
        activeDisplayId: ContainerId;
    } = $props();

    let workspace: HTMLElement = null;
    let control: {
        layoutAndIndex: [Layout, number];
        initialValue: number;
        initialPosition: number;
        dimension: number;
        isVertical: boolean;
    } = $state({
        layoutAndIndex: null,
        initialValue: 0.0,
        initialPosition: 0.0,
        dimension: 0.0,
        isVertical: false,
    });

    function mouseDown(event: MouseEvent, layout: Layout, index: number) {
        control.layoutAndIndex = [layout, index];
        control.initialValue =
            appState.local.layoutToPosition[control.layoutAndIndex[0]][
                control.layoutAndIndex[1]
            ];
        switch (layout) {
            case "h":
                control.isVertical = false;
                break;
            case "hv1":
            case "hv2":
            case "hv1v2":
                control.isVertical = index > 0;
                break;
            case "v":
                control.isVertical = true;
                break;
            case "vh1":
            case "vh2":
            case "vh1h2":
                control.isVertical = index === 0;
                break;
            default:
                break;
        }
        if (control.isVertical) {
            control.initialPosition = event.clientX;
            control.dimension = workspace.clientWidth;
        } else {
            control.initialPosition = event.clientY;
            control.dimension = workspace.clientHeight;
        }
    }

    function mouseMove(event: MouseEvent) {
        if (control.layoutAndIndex != null) {
            let position;
            if (control.isVertical) {
                position = event.clientX;
            } else {
                position = event.clientY;
            }
            appState.local.layoutToPosition[control.layoutAndIndex[0]][
                control.layoutAndIndex[1]
            ] = Math.min(
                0.9,
                Math.max(
                    0.1,
                    control.initialValue +
                        (position - control.initialPosition) /
                            control.dimension,
                ),
            );
        }
    }

    function mouseUp() {
        control.layoutAndIndex = null;
    }
</script>

<svelte:window onmouseup={mouseUp} onmousemove={mouseMove} />

<div class="workspace" bind:this={workspace}>
    {#if appState.local.layout === "full"}
        <Container id={1} size={1.0} bind:activeId={activeDisplayId}
        ></Container>
    {:else if appState.local.layout === "h"}
        <div class="h">
            <Container
                id={1}
                size={appState.local.layoutToPosition.h[0]}
                bind:activeId={activeDisplayId}
            ></Container>
            <Container
                id={2}
                size={1.0 - appState.local.layoutToPosition.h[0]}
                bind:activeId={activeDisplayId}
            ></Container>
        </div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({appState.local.layoutToPosition
                .h[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "h", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "hv1"}
        <div class="h">
            <div
                class="v"
                style="flex-grow: {appState.local.layoutToPosition.hv1[0]}"
            >
                <Container
                    id={1}
                    size={appState.local.layoutToPosition.hv1[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.hv1[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
            <Container
                id={2}
                size={1.0 - appState.local.layoutToPosition.hv1[0]}
                bind:activeId={activeDisplayId}
            ></Container>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: {(1.0 -
                appState.local.layoutToPosition.hv1[0]) *
                100}%; left: calc({appState.local.layoutToPosition.hv1[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({appState.local.layoutToPosition
                .hv1[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "hv2"}
        <div class="h">
            <Container
                id={1}
                size={appState.local.layoutToPosition.hv2[0]}
                bind:activeId={activeDisplayId}
            ></Container>
            <div
                class="v"
                style="flex-grow: {1.0 -
                    appState.local.layoutToPosition.hv2[0]}"
            >
                <Container
                    id={2}
                    size={appState.local.layoutToPosition.hv2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.hv2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
        </div>
        <div
            class="control-v"
            style="top: {appState.local.layoutToPosition.hv2[0] *
                100}%; bottom: 0; left: calc({appState.local.layoutToPosition
                .hv2[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv2", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({appState.local.layoutToPosition
                .hv2[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv2", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "v"}
        <div class="v">
            <Container
                id={1}
                size={appState.local.layoutToPosition.v[0]}
                bind:activeId={activeDisplayId}
            ></Container>
            <Container
                id={2}
                size={1.0 - appState.local.layoutToPosition.v[0]}
                bind:activeId={activeDisplayId}
            ></Container>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({appState.local
                .layoutToPosition.v[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "v", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "vh1"}
        <div class="v">
            <div
                class="h"
                style="flex-grow: {appState.local.layoutToPosition.vh1[0]}"
            >
                <Container
                    id={1}
                    size={appState.local.layoutToPosition.vh1[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.vh1[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
            <Container
                id={2}
                size={1.0 - appState.local.layoutToPosition.vh1[0]}
                bind:activeId={activeDisplayId}
            ></Container>
        </div>
        <div
            class="control-h"
            style="left: 0; right: {(1.0 -
                appState.local.layoutToPosition.vh1[0]) *
                100}%; top: calc({appState.local.layoutToPosition.vh1[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({appState.local
                .layoutToPosition.vh1[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "vh2"}
        <div class="v">
            <Container
                id={1}
                size={appState.local.layoutToPosition.vh2[0]}
                bind:activeId={activeDisplayId}
            ></Container>
            <div
                class="h"
                style="flex-grow: {1.0 -
                    appState.local.layoutToPosition.vh2[0]}"
            >
                <Container
                    id={2}
                    size={appState.local.layoutToPosition.vh2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.vh2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
        </div>
        <div
            class="control-h"
            style="left: {appState.local.layoutToPosition.vh2[0] *
                100}%; right: 0; top: calc({appState.local.layoutToPosition
                .vh2[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh2", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({appState.local
                .layoutToPosition.vh2[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh2", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "hv1v2"}
        <div class="h">
            <div
                class="v"
                style="flex-grow: {appState.local.layoutToPosition.hv1v2[0]}"
            >
                <Container
                    id={1}
                    size={appState.local.layoutToPosition.hv1v2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={4}
                    size={1.0 - appState.local.layoutToPosition.hv1v2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
            <div
                class="v"
                style="flex-grow: {1.0 -
                    appState.local.layoutToPosition.hv1v2[0]}"
            >
                <Container
                    id={2}
                    size={appState.local.layoutToPosition.hv1v2[2]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.hv1v2[2]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: {(1.0 -
                appState.local.layoutToPosition.hv1v2[0]) *
                100}%; left: calc({appState.local.layoutToPosition.hv1v2[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: {appState.local.layoutToPosition.hv1v2[0] *
                100}%; bottom: 0; left: calc({appState.local.layoutToPosition
                .hv1v2[2] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 2)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({appState.local.layoutToPosition
                .hv1v2[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 0)}
            role="none"
        ></div>
    {:else if appState.local.layout === "vh1h2"}
        <div class="v">
            <div
                class="h"
                style="flex-grow: {appState.local.layoutToPosition.vh1h2[0]}"
            >
                <Container
                    id={1}
                    size={appState.local.layoutToPosition.vh1h2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={4}
                    size={1.0 - appState.local.layoutToPosition.vh1h2[1]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
            <div
                class="h"
                style="flex-grow: {1.0 -
                    appState.local.layoutToPosition.vh1h2[0]}"
            >
                <Container
                    id={2}
                    size={appState.local.layoutToPosition.vh1h2[2]}
                    bind:activeId={activeDisplayId}
                ></Container>
                <Container
                    id={3}
                    size={1.0 - appState.local.layoutToPosition.vh1h2[2]}
                    bind:activeId={activeDisplayId}
                ></Container>
            </div>
        </div>
        <div
            class="control-h"
            style="left: 0; right: {(1.0 -
                appState.local.layoutToPosition.vh1h2[0]) *
                100}%; top: calc({appState.local.layoutToPosition.vh1h2[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1h2", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: {appState.local.layoutToPosition.vh1h2[0] *
                100}%; right: 0; top: calc({appState.local.layoutToPosition
                .vh1h2[2] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1h2", 2)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({appState.local
                .layoutToPosition.vh1h2[0] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1h2", 0)}
            role="none"
        ></div>
    {/if}
</div>

<style>
    .workspace {
        background-color: var(--background-0);
        height: calc(100vh - 6 - var(--status-bar-height));
        flex-grow: 1;
        display: flex;
        position: relative;
        margin: 2px;
    }

    .control-h {
        position: absolute;
        z-index: 5;
        height: 12px;
        opacity: 0;
        cursor: row-resize;
    }

    .control-v {
        position: absolute;
        z-index: 5;
        width: 12px;
        opacity: 0;
        cursor: col-resize;
    }

    .h {
        flex-grow: 1;
        flex-basis: 0;
        display: flex;
        flex-direction: column;
        align-items: stretch;
        align-content: stretch;
        gap: 1px;
    }

    .v {
        flex-grow: 1;
        flex-basis: 0;
        display: flex;
        flex-direction: row;
        align-items: stretch;
        align-content: stretch;
        gap: 1px;
    }
</style>
