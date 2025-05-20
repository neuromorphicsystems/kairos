<script lang="ts">
    import type { ContainerId, Layout } from "./constants";
    import Container from "./container.svelte";

    let {
        layout,
        activeContainerId = $bindable(),
        displayPaneOpen = $bindable(),
    }: {
        activeContainerId: ContainerId;
        displayPaneOpen: boolean;
        layout: Layout;
    } = $props();
    const layoutToPosition: { [key in Layout]: number[] } = $state({
        full: [],
        h: [0.5],
        hv1: [0.5, 0.5],
        hv2: [0.5, 0.5],
        v: [0.5],
        vh1: [0.5, 0.5],
        vh2: [0.5, 0.5],
        hv1v2: [0.5, 0.5, 0.5],
        vh1h2: [0.5, 0.5, 0.5],
    });

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
            layoutToPosition[control.layoutAndIndex[0]][
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
            layoutToPosition[control.layoutAndIndex[0]][
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
    {#if layout === "full"}
        <Container
            id={1}
            size={1.0}
            bind:activeId={activeContainerId}
            bind:displayPaneOpen
        >
        </Container>
    {:else if layout === "h"}
        <div class="h">
            <Container
                id={1}
                size={layoutToPosition.h[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
            <Container
                id={2}
                size={1.0 - layoutToPosition.h[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
        </div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({layoutToPosition.h[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "h", 0)}
            role="none"
        ></div>
    {:else if layout === "hv1"}
        <div class="h">
            <div class="v" style="flex-grow: {layoutToPosition.hv1[0]}">
                <Container
                    id={2}
                    size={layoutToPosition.hv1[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={3}
                    size={1.0 - layoutToPosition.hv1[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
            <Container
                id={1}
                size={1.0 - layoutToPosition.hv1[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: {(1.0 - layoutToPosition.hv1[0]) *
                100}%; left: calc({layoutToPosition.hv1[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({layoutToPosition.hv1[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1", 0)}
            role="none"
        ></div>
    {:else if layout === "hv2"}
        <div class="h">
            <Container
                id={1}
                size={layoutToPosition.hv2[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
            <div class="v" style="flex-grow: {1.0 - layoutToPosition.hv2[0]}">
                <Container
                    id={2}
                    size={layoutToPosition.hv2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={3}
                    size={1.0 - layoutToPosition.hv2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
        </div>
        <div
            class="control-v"
            style="top: {layoutToPosition.hv2[0] *
                100}%; bottom: 0; left: calc({layoutToPosition.hv2[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv2", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({layoutToPosition.hv2[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv2", 0)}
            role="none"
        ></div>
    {:else if layout === "v"}
        <div class="v">
            <Container
                id={1}
                size={layoutToPosition.v[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
            <Container
                id={2}
                size={1.0 - layoutToPosition.v[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({layoutToPosition.v[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "v", 0)}
            role="none"
        ></div>
    {:else if layout === "vh1"}
        <div class="v">
            <div class="h" style="flex-grow: {layoutToPosition.vh1[0]}">
                <Container
                    id={2}
                    size={layoutToPosition.vh1[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={3}
                    size={1.0 - layoutToPosition.vh1[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
            <Container
                id={1}
                size={1.0 - layoutToPosition.vh1[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
        </div>
        <div
            class="control-h"
            style="left: 0; right: {(1.0 - layoutToPosition.vh1[0]) *
                100}%; top: calc({layoutToPosition.vh1[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({layoutToPosition.vh1[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1", 0)}
            role="none"
        ></div>
    {:else if layout === "vh2"}
        <div class="v">
            <Container
                id={1}
                size={layoutToPosition.vh2[0]}
                bind:activeId={activeContainerId}
                bind:displayPaneOpen
            ></Container>
            <div class="h" style="flex-grow: {1.0 - layoutToPosition.vh2[0]}">
                <Container
                    id={2}
                    size={layoutToPosition.vh2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={3}
                    size={1.0 - layoutToPosition.vh2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
        </div>
        <div
            class="control-h"
            style="left: {layoutToPosition.vh2[0] *
                100}%; right: 0; top: calc({layoutToPosition.vh2[1] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh2", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({layoutToPosition.vh2[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh2", 0)}
            role="none"
        ></div>
    {:else if layout === "hv1v2"}
        <div class="h">
            <div class="v" style="flex-grow: {layoutToPosition.hv1v2[0]}">
                <Container
                    id={1}
                    size={layoutToPosition.hv1v2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={2}
                    size={1.0 - layoutToPosition.hv1v2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
            <div class="v" style="flex-grow: {1.0 - layoutToPosition.hv1v2[0]}">
                <Container
                    id={3}
                    size={layoutToPosition.hv1v2[2]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={4}
                    size={1.0 - layoutToPosition.hv1v2[2]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
        </div>
        <div
            class="control-v"
            style="top: 0; bottom: {(1.0 - layoutToPosition.hv1v2[0]) *
                100}%; left: calc({layoutToPosition.hv1v2[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 1)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: {layoutToPosition.hv1v2[0] *
                100}%; bottom: 0; left: calc({layoutToPosition.hv1v2[2] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 2)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: 0; right: 0; top: calc({layoutToPosition.hv1v2[0] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "hv1v2", 0)}
            role="none"
        ></div>
    {:else if layout === "vh1h2"}
        <div class="v">
            <div class="h" style="flex-grow: {layoutToPosition.vh1h2[0]}">
                <Container
                    id={1}
                    size={layoutToPosition.vh1h2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={2}
                    size={1.0 - layoutToPosition.vh1h2[1]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
            <div class="h" style="flex-grow: {1.0 - layoutToPosition.vh1h2[0]}">
                <Container
                    id={3}
                    size={layoutToPosition.vh1h2[2]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
                <Container
                    id={4}
                    size={1.0 - layoutToPosition.vh1h2[2]}
                    bind:activeId={activeContainerId}
                    bind:displayPaneOpen
                ></Container>
            </div>
        </div>
        <div
            class="control-h"
            style="left: 0; right: {(1.0 - layoutToPosition.vh1h2[0]) *
                100}%; top: calc({layoutToPosition.vh1h2[1] * 100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1h2", 1)}
            role="none"
        ></div>
        <div
            class="control-h"
            style="left: {layoutToPosition.vh1h2[0] *
                100}%; right: 0; top: calc({layoutToPosition.vh1h2[2] *
                100}% - 6px);"
            onmousedown={event => mouseDown(event, "vh1h2", 2)}
            role="none"
        ></div>
        <div
            class="control-v"
            style="top: 0; bottom: 0; left: calc({layoutToPosition.vh1h2[0] *
                100}% - 6px);"
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
        z-index: 1;
        height: 12px;
        opacity: 0;
        cursor: row-resize;
    }

    .control-v {
        position: absolute;
        z-index: 1;
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
