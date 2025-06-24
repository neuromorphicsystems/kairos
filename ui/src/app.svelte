<script lang="ts">
    import { Toaster } from "svelte-sonner";

    import type { ContainerId } from "./constants";

    import {} from "./protocol.svelte";
    import appState from "./appState.svelte";
    import DisplayPane from "./displayPane.svelte";
    import StatusBar from "./statusBar.svelte";
    import DevicePane from "./devicePane.svelte";
    import Workspace from "./workspace.svelte";

    let devicePaneOpen: boolean = $state(false);
    let displayPaneOpen: boolean = $state(false);
    let activeDisplayId: ContainerId = $state(0);
</script>

<main>
    <StatusBar bind:devicePaneOpen bind:displayPaneOpen bind:activeDisplayId
    ></StatusBar>
    <div class="content">
        <DevicePane open={devicePaneOpen}></DevicePane>
        <Workspace bind:activeDisplayId></Workspace>
        <DisplayPane open={displayPaneOpen} {activeDisplayId}></DisplayPane>
    </div>
    <Toaster
        closeButton
        toastOptions={{
            unstyled: true,
            classes: {
                toast: "toast",
                error: "error",
                success: "success",
                warning: "warning",
                info: "info",
                title: "toast-title",
                description: "toast-description",
                closeButton: "toast-close-button",
            },
        }}
    />
</main>

<style>
    :root {
        overscroll-behavior: none;
        --status-bar-height: 50px;
        --device-pane-width: 300px;
        --display-pane-width: 300px;
        --recordings-width: 600px;
        --background-0: #000000;
        --background-1: #202020;
        --background-2: #282828;
        --background-3: #333333;
        --button-background: #484848;
        --button-background-hover: #525252;
        --button-background-active: #5c5c5c;
        --border: #555555;
        --blue-0: #72a0c7;
        --blue-1: #4f88b9;
        --red-1: #874037;
        --yellow-1: #c3a34b;
        --content-0: #aaaaaa;
        --content-1: #bbbbbb;
        --content-2: #cccccc;
        --content-3: #dddddd;
    }

    .content {
        display: flex;
        background-color: var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        overflow: hidden;
    }

    main :global(.toast) {
        border: 1px solid var(--border);
        border-radius: 8px;
        box-shadow: 0 0 8px 0 #00000080;
        padding: 10px;
        display: flex;
        gap: 10px;
        align-items: center;
    }

    main :global(.toast.error) {
        background-color: var(--red-1);
        border-color: var(--content-0);
    }

    main :global(.toast.success) {
        background-color: var(--blue-1);
        border-color: var(--content-0);
    }

    main :global(.toast.warning) {
        background-color: var(--yellow-1);
        border-color: var(--content-0);
    }

    main :global(.toast.info) {
        background-color: var(--background-3);
        border-color: var(--content-0);
    }

    main :global(.toast svg) {
        display: block;
        fill: var(--content-3);
    }

    main :global(.toast-title) {
        font-size: 14px;
        color: var(--content-3);
    }

    main :global(.toast-description) {
        font-size: 14px;
        color: var(--content-3);
    }

    main :global(.toast-close-button) {
        position: absolute;
        left: 0;
        top: 0;
        height: 24px;
        width: 24px;
        display: flex;
        justify-content: center;
        align-items: center;
        padding: 0;
        background-color: var(--background-3);
        border: 1px solid var(--content-0);
        border-color: var(--content-0);
        transform: translate(-35%, -35%);
        border-radius: 50%;
        cursor: pointer;
        z-index: 1;
    }

    main :global(.toast-close-button:hover) {
        background-color: var(--background-0);
    }

    main :global(.toast-close-button svg) {
        stroke: var(--content-3);
    }
</style>
