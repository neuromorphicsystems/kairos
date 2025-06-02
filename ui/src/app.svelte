<script lang="ts">
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
    <StatusBar
        bind:devicePaneOpen
        bind:displayPaneOpen
        bind:activeDisplayId
    ></StatusBar>
    <div class="content">
        <DevicePane open={devicePaneOpen}></DevicePane>
        <Workspace bind:activeDisplayId></Workspace>
        <DisplayPane open={displayPaneOpen} {activeDisplayId}></DisplayPane>
    </div>
</main>

<style>
    :root {
        --status-bar-height: 50px;
        --device-pane-width: 300px;
        --display-pane-width: 300px;
        --background-0: #000000;
        --background-1: #202020;
        --background-2: #282828;
        --background-3: #333333;
        --blue-0: #174ea6;
        --red-0: #a50e0e;
        --yellow-0: #e37400;
        --green-0: #0d652d;
        --blue-1: #4285f4;
        --red-1: #ea4335;
        --yellow-1: #fbbc04;
        --green-1: #34a853;
    }

    .content {
        display: flex;
        background-color: var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        overflow: hidden;
    }
</style>
