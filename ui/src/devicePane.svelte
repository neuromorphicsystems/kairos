<script lang="ts">
    import {
        startRecording,
        stopRecording,
        updateAutostop,
        updateAutotrigger,
        updateConfiguration,
        updateLookback,
    } from "./protocol.svelte";
    import appState from "./appState.svelte";
    import * as constants from "./constants";
    import * as deviceConfiguration from "./deviceConfiguration";
    import * as utilities from "./utilities";
    import Button from "./button.svelte";
    import Dropdown from "./dropdown.svelte";
    import NumberInput from "./numberInput.svelte";
    import NumberInputWithSlider from "./numberInputWithSlider.svelte";
    import Switch from "./switch.svelte";

    const { open }: { open: boolean } = $props();

    const groupsAndParameters = $derived(
        appState.local.deviceIndex == null
            ? null
            : deviceConfiguration.groupsAndParameters(
                  appState.shared.devices[appState.local.deviceIndex]
                      .configuration,
              ),
    );
    const recordState = $derived(
        appState.local.deviceIndex == null
            ? null
            : appState.deviceIdToRecordState[
                  appState.shared.devices[appState.local.deviceIndex].id
              ],
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
                    <div class="value monospace">
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
                    <div class="value">
                        {appState.shared.data_directory}
                    </div>
                </div>
                <div class="property">
                    <div class="name">Available space</div>
                    <div class="value monospace">
                        {appState.shared.disk_available_and_total_space == null
                            ? "?"
                            : `${utilities.gbSizeToString(
                                  appState.shared
                                      .disk_available_and_total_space[0],
                              )} / ${utilities.gbSizeToString(
                                  appState.shared
                                      .disk_available_and_total_space[1],
                              )}`}
                    </div>
                </div>
            </div>

            {#if recordState != null}
                <div class="horizontal-line"></div>
                <div class="record">
                    <div
                        class="buttons"
                        style="justify-content: {recordState.recording == null
                            ? 'left'
                            : 'right'}"
                    >
                        {#if recordState.recording == null}
                            <Button
                                label="Start recording"
                                icon={recordIcon}
                                iconStyle="width: 10px; height: 10px"
                                onClick={() => {
                                    startRecording(
                                        appState.shared.devices[
                                            appState.local.deviceIndex
                                        ].id,
                                        "",
                                    );
                                }}
                            ></Button>
                        {:else}
                            <Button
                                label="Stop recording"
                                icon={stopIcon}
                                iconStyle="width: 10px; height: 10px"
                                onClick={() => {
                                    stopRecording(
                                        appState.shared.devices[
                                            appState.local.deviceIndex
                                        ].id,
                                    );
                                }}
                            ></Button>
                        {/if}
                    </div>
                    {#if recordState.recording != null}
                        <div class="properties">
                            <div class="property">
                                <div class="name">Recording name</div>
                                <div class="value">
                                    {recordState.recording.name}
                                </div>
                            </div>
                            <div class="property">
                                <div class="name">Recording duration</div>
                                <div class="value monospace">
                                    {utilities.durationToString(
                                        recordState.recording.duration_us,
                                    )}
                                </div>
                            </div>
                            <div class="property">
                                <div class="name">Recording size</div>
                                <div class="value monospace">
                                    {utilities.sizeToString(
                                        recordState.recording.size_bytes,
                                    )}
                                </div>
                            </div>
                        </div>
                    {/if}
                </div>
            {/if}

            <div class="horizontal-line"></div>
            <Switch
                label={"Lookback"}
                checked={appState.shared.devices[appState.local.deviceIndex]
                    .lookback.enabled}
                onChange={enabled => {
                    updateLookback(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].lookback,
                            ),
                            enabled,
                        },
                    );
                }}
                labelWidth={60}
            ></Switch>
            <NumberInputWithSlider
                label="Maximum duration"
                description="Maximum duration of the lookback buffer, in milliseconds. The actual duration is also limited by the maximum size."
                rightSide={false}
                value={Math.round(
                    appState.shared.devices[appState.local.deviceIndex].lookback
                        .maximum_duration_us / 1000,
                )}
                units="ms"
                step={1}
                min={50}
                max={200000}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={0}
                paddingTop={10}
                onChange={newMaximumDurationMs => {
                    updateLookback(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].lookback,
                            ),
                            maximum_duration_us: newMaximumDurationMs * 1000,
                        },
                    );
                }}
                sliderLogarithmic={true}
                sliderTicks={[100, 1000, 10000, 100000]}
                sliderWidth={260}
            ></NumberInputWithSlider>
            <NumberInputWithSlider
                label="Maximum size"
                description="Maximum size of the lookback buffer, in megabytes. The actual size is also limited by the maximum duration."
                rightSide={false}
                value={Math.round(
                    appState.shared.devices[appState.local.deviceIndex].lookback
                        .maximum_size_bytes / 1e6,
                )}
                units="MB"
                step={1}
                min={256}
                max={16384}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={0}
                paddingTop={20}
                onChange={newMaximumSizeMb => {
                    updateLookback(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].lookback,
                            ),
                            maximum_size_bytes: newMaximumSizeMb * 1e6,
                        },
                    );
                }}
                sliderLogarithmic={true}
                sliderTicks={[512, 1024, 2048, 4096, 8192]}
                sliderWidth={260}
            ></NumberInputWithSlider>
            {#if recordState != null && recordState.lookback != null}
                <div class="properties">
                    <div class="property">
                        <div class="name">Buffer duration</div>
                        <div class="value monospace">
                            {utilities.durationToString(
                                recordState.lookback.duration_us,
                            )}
                        </div>
                    </div>
                    <div class="property">
                        <div class="name">Buffer size</div>
                        <div class="value monospace">
                            {utilities.sizeToString(
                                recordState.lookback.size_bytes,
                            )}
                        </div>
                    </div>
                </div>
            {/if}

            <div class="horizontal-line"></div>
            <Switch
                label={"Autostop"}
                checked={appState.shared.devices[appState.local.deviceIndex]
                    .autostop.enabled}
                onChange={enabled => {
                    updateAutostop(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autostop,
                            ),
                            enabled,
                        },
                    );
                }}
                labelWidth={56}
            ></Switch>
            <NumberInputWithSlider
                label="Duration"
                description="Duration after which recording automatically stops, in milliseconds. Autostop considers the time after the trigger (whether manual or auto), independetly of the lookback. For instance, a recording with a lookback of 10 s and an autostop of 5 s will last 15 s."
                rightSide={false}
                value={Math.round(
                    appState.shared.devices[appState.local.deviceIndex].autostop
                        .duration_us / 1000,
                )}
                units="ms"
                step={1}
                min={50}
                max={200000}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={0}
                paddingTop={10}
                onChange={newMaximumDurationMs => {
                    updateAutostop(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autostop,
                            ),
                            duration_us: newMaximumDurationMs * 1000,
                        },
                    );
                }}
                sliderLogarithmic={true}
                sliderTicks={[100, 1000, 10000, 100000]}
                sliderWidth={260}
            ></NumberInputWithSlider>

            <div class="horizontal-line"></div>
            <Switch
                label={"Autotrigger"}
                checked={appState.shared.devices[appState.local.deviceIndex]
                    .autotrigger.enabled}
                onChange={enabled => {
                    updateAutotrigger(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autotrigger,
                            ),
                            enabled,
                        },
                    );
                }}
                labelWidth={70}
            ></Switch>
            <NumberInput
                label="Short sliding window"
                description={"Number of event rate samples in the short sliding window, which detects sudden changes. The event rate is computed at 60 Hz, the sliding window duration is this number times 1/60 s."}
                rightSide={false}
                value={Math.round(
                    appState.shared.devices[appState.local.deviceIndex]
                        .autotrigger.short_sliding_window,
                )}
                units={null}
                step={1}
                min={1}
                max={constants.AUTOTRIGGER_MAXIMUM_WINDOW_SIZE}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={0}
                paddingTop={10}
                onChange={short_sliding_window => {
                    updateAutotrigger(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autotrigger,
                            ),
                            short_sliding_window,
                        },
                    );
                }}
            ></NumberInput>
            <NumberInput
                label="Long sliding window"
                description={"Number of event rate samples in the long sliding window, which computes the baseline. The event rate is computed at 60 Hz, the sliding window duration is this number times 1/60 s."}
                rightSide={false}
                value={appState.shared.devices[appState.local.deviceIndex]
                    .autotrigger.long_sliding_window}
                units={null}
                step={1}
                min={1}
                max={constants.AUTOTRIGGER_MAXIMUM_WINDOW_SIZE}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={0}
                paddingTop={10}
                onChange={long_sliding_window => {
                    updateAutotrigger(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autotrigger,
                            ),
                            long_sliding_window,
                        },
                    );
                }}
            ></NumberInput>
            <NumberInput
                label="Threshold"
                description={'When the ratio of the <em>short</em> average divided by the <em>long</em> average becomes larger that this value, a new recording starts. If the system is already recording and autostop is enabled, the autostop counter is reset. If autostop is not enabled, the system records until the user clicks "Stop recording".'}
                rightSide={false}
                value={appState.shared.devices[appState.local.deviceIndex]
                    .autotrigger.threshold}
                units={null}
                step={"any"}
                min={1.0}
                max={1000.0}
                labelWidth={160}
                labelBold={false}
                inputWidth={100}
                digits={3}
                paddingTop={10}
                onChange={threshold => {
                    updateAutotrigger(
                        appState.shared.devices[appState.local.deviceIndex].id,
                        {
                            ...$state.snapshot(
                                appState.shared.devices[
                                    appState.local.deviceIndex
                                ].autotrigger,
                            ),
                            threshold,
                        },
                    );
                }}
            ></NumberInput>

            {#each groupsAndParameters as group_and_parameters}
                <div class="horizontal-line"></div>
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

    .horizontal-line {
        margin-top: 20px;
        height: 1px;
        background-color: var(--border);
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
        align-items: baseline;
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

    .properties .property .value.monospace {
        font-family: "RobotoMono", monospace;
        font-size: 12px;
    }

    .record {
        padding-top: 10px;
    }

    .buttons {
        display: flex;
    }

    .record .properties {
        padding-top: 10px;
    }
</style>
