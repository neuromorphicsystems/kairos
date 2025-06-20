<script lang="ts">
    import {
        startRecording,
        stopRecording,
        updateConfiguration,
    } from "./protocol.svelte";
    import * as deviceConfiguration from "./deviceConfiguration";
    import * as utilities from "./utilities";
    import Button from "./button.svelte";
    import Dropdown from "./dropdown.svelte";
    import NumberInput from "./numberInput.svelte";
    import appState from "./appState.svelte";

    const { open }: { open: boolean } = $props();

    const groupsAndParameters = $derived(
        appState.local.deviceIndex == null
            ? null
            : deviceConfiguration.groupsAndParameters(
                  appState.shared.devices[appState.local.deviceIndex]
                      .configuration,
              ),
    );
</script>

<div class="device-pane {open ? 'open' : ''}">
    <div class="content">
        {#if appState.local.deviceIndex == null}
            <div class="message">There are no connected devices.</div>
        {:else}
            <div class="label">Device</div>
            <Dropdown
                choices={appState.shared.devices.map(device => [
                    `${device.serial} (${device.name})`,
                    null,
                ])}
                bind:selectedIndex={appState.local.deviceIndex}
            ></Dropdown>

            <div class="properties">
                <div class="property">
                    <div class="name">Speed</div>
                    <div class="value">
                        {appState.shared.devices[appState.local.deviceIndex]
                            .speed}
                    </div>
                </div>
                <div class="property">
                    <div class="name">Bus and address</div>
                    <div class="value">
                        {appState.shared.devices[
                            appState.local.deviceIndex
                        ].bus_number
                            .toFixed(0)
                            .padStart(3, "0")}:{appState.shared.devices[
                            appState.local.deviceIndex
                        ].address
                            .toFixed(0)
                            .padStart(3, "0")}
                    </div>
                </div>
            </div>

            <div class="properties">
                <div class="property">
                    <div class="name">Data directory</div>
                    <div class="value directory">
                        {appState.shared.data_directory}
                    </div>
                </div>
                <div class="property">
                    <div class="name">Available space</div>
                    <div class="value">
                        {appState.shared.disk_available_and_total_space == null
                            ? "?"
                            : `${utilities.size(appState.shared.disk_available_and_total_space[0])} / ${utilities.size(appState.shared.disk_available_and_total_space[1])}`}
                    </div>
                </div>
            </div>

            <div class="record">
                <Button
                    label="Start recording"
                    icon={recordIcon}
                    iconStyle="width: 10px; height: 10px"
                    onClick={() => {
                        startRecording(
                            appState.shared.devices[appState.local.deviceIndex]
                                .id,
                            "",
                        );
                    }}
                ></Button>
                <Button
                    label="Stop recording"
                    icon={stopIcon}
                    iconStyle="width: 10px; height: 10px"
                    onClick={() => {
                        stopRecording(
                            appState.shared.devices[appState.local.deviceIndex]
                                .id,
                        );
                    }}
                ></Button>
            </div>

            {#each groupsAndParameters as group_and_parameters}
                <div class="group-label">
                    {group_and_parameters[0]}
                </div>
                <div class="group">
                    {#each group_and_parameters[1] as parameter, index}
                        {#if parameter.type === "integer"}
                            <NumberInput
                                label="{parameter.name} {parameter.value ===
                                parameter.default
                                    ? ''
                                    : '&#x25cf;'}"
                                description={[
                                    parameter.description,
                                    `<i>Default</i><code>${parameter.default}</code>`,
                                    `<i>Range</i><code>[${parameter.minimum}, ${parameter.maximum}]</code>`,
                                ]
                                    .map(paragraph => `<p>${paragraph}</p>`)
                                    .join("")}
                                rightSide={false}
                                bind:value={parameter.value}
                                units={null}
                                step={"any"}
                                min={parameter.minimum}
                                max={parameter.maximum}
                                labelWidth={160}
                                labelBold={parameter.value !==
                                    parameter.default}
                                inputWidth={100}
                                digits={0}
                                paddingTop={index === 0 ? 0 : 10}
                                onChange={newValue => {
                                    parameter.update(newValue);
                                    updateConfiguration(
                                        appState.shared.devices[
                                            appState.local.deviceIndex
                                        ].id,
                                        appState.shared.devices[
                                            appState.local.deviceIndex
                                        ].configuration,
                                    );
                                }}
                            ></NumberInput>
                        {/if}
                    {/each}
                </div>
            {/each}
        {/if}
    </div>
</div>

{#snippet recordIcon()}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
        ><circle
            cx="50"
            cy="50"
            r="50"
            fill="#DDDDDD"
            fill-rule="evenodd"
        /></svg
    >
{/snippet}

{#snippet stopIcon()}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
        ><rect
            width="100"
            height="100"
            fill="#DDDDDD"
            fill-rule="evenodd"
        /></svg
    >
{/snippet}

<style>
    .device-pane {
        width: 0;
        border-top: 1px solid var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        transition: width 0.3s;
        overflow: hidden;
        position: relative;
    }

    .device-pane.open {
        width: var(--device-pane-width);
    }

    .content {
        width: var(--device-pane-width);
        height: calc(100vh - 1px - var(--status-bar-height));
        background-color: var(--background-2);
        position: absolute;
        right: 0;
        top: 0;
        padding: 20px;
        overflow-y: auto;
        overflow-x: hidden;
    }

    .message {
        font-size: 14px;
        color: var(--content-0);
    }

    .label {
        font-size: 14px;
        padding-bottom: 5px;
        color: var(--content-0);
    }

    .group-label {
        padding-top: 20px;
        padding-bottom: 10px;
        font-size: 14px;
        color: var(--content-0);
    }

    .label:not(:first-of-type) {
        padding-top: 20px;
    }

    .properties {
        padding-top: 20px;
        display: flex;
        flex-direction: column;
        gap: 5px;
    }

    .properties .property {
        font-size: 14px;
        display: flex;
        flex-wrap: wrap;
        gap: 5px 10px;
    }

    .properties .property .name {
        flex-grow: 0;
        flex-shrink: 0;
        color: var(--content-0);
    }

    .properties .property .value {
        color: var(--content-2);
        overflow-wrap: break-word;
        max-width: calc(var(--device-pane-width) - 40px);
    }

    .record {
        padding-top: 10px;
    }
</style>
