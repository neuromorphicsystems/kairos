<script lang="ts">
    import type { ContainerId } from "./constants";
    import type { SampleDisplayProperties, Stream } from "./appState.svelte";

    import appState, {
        defaultEventDisplayProperties,
        defaultSampleDisplayProperties,
        defaultNamesAndRanges,
    } from "./appState.svelte";
    import * as constants from "./constants";
    import * as utilities from "./utilities";
    import { namedColormaps } from "./colormaps";
    import Dropdown from "./dropdown.svelte";
    import Switch from "./switch.svelte";

    const {
        open,
        activeDisplayId,
    }: { open: boolean; activeDisplayId: ContainerId } = $props();

    interface Source {
        label: string;
        stream: Stream;
        propertiesType: "EventDisplayProperties" | "SampleDisplayProperties";
        target: {
            deviceId: number;
            streamIndex: number;
        };
    }

    const display = $derived(
        activeDisplayId === 0
            ? null
            : appState.local.displays[activeDisplayId - 1],
    );
    const [sources, selectedIndex] = $derived.by(() => {
        const sources: Source[] = [];
        let selectedIndex: number = null;
        let index = 0;
        for (const device of appState.shared.devices) {
            let streamIndex = 0;
            for (const stream of device.streams) {
                if (
                    display.target != null &&
                    display.target.deviceId === device.id &&
                    display.target.streamIndex === streamIndex
                ) {
                    selectedIndex = index;
                }
                sources.push({
                    label: `${device.name} ${device.serial} – ${stream.type === "Evt3" ? "Events" : "Samples"}`,
                    stream,
                    propertiesType:
                        stream.type === "Evt3"
                            ? "EventDisplayProperties"
                            : "SampleDisplayProperties",
                    target: { deviceId: device.id, streamIndex },
                });
                ++index;
                ++streamIndex;
            }
        }
        if (selectedIndex == null) {
            if (display.properties.type === "EventDisplayProperties") {
                selectedIndex = index;
            }
            sources.push({
                label: "Any source – Events",
                stream: null,
                propertiesType: "EventDisplayProperties",
                target: null,
            });
            ++index;
            if (display.properties.type === "SampleDisplayProperties") {
                selectedIndex = index;
            }
            sources.push({
                label: "Any source – Samples",
                stream: null,
                propertiesType: "SampleDisplayProperties",
                target: null,
            });
            ++index;
        }
        if (selectedIndex == null) {
            throw new Error(
                `selectedIndex is null, sources=${JSON.stringify(sources)}, activeDisplayId=${activeDisplayId}, appState=${JSON.stringify($state.snapshot(appState))}`,
            );
        }
        return [sources, selectedIndex];
    });
</script>

<div class="display-pane {open ? 'open' : ''}">
    <div class="content">
        {#if display == null}
            <div class="message">
                Select a layout item to edit its contents and style.
            </div>
        {:else}
            <Dropdown
                choices={sources.map(source => source.label)}
                {selectedIndex}
                onChange={newSelectedIndex => {
                    const newSource = sources[newSelectedIndex];
                    if (display.properties.type === newSource.propertiesType) {
                        if (
                            newSource.propertiesType ===
                            "SampleDisplayProperties"
                        ) {
                            const properties = appState.local.displays[
                                activeDisplayId - 1
                            ].properties as SampleDisplayProperties;
                            const previousNamesAndRanges =
                                properties.namesAndRangesIndices;
                            properties.namesAndRangesIndices =
                                defaultNamesAndRanges(newSource.stream);
                            for (const nameAndRange of properties.namesAndRangesIndices) {
                                for (const [
                                    previousName,
                                    previousRange,
                                ] of previousNamesAndRanges) {
                                    if (nameAndRange[0] === previousName) {
                                        nameAndRange[1] = previousRange;
                                    }
                                }
                            }
                            properties.hash = utilities.nextUnique();
                        }
                    } else {
                        switch (newSource.propertiesType) {
                            case "EventDisplayProperties": {
                                appState.local.displays[
                                    activeDisplayId - 1
                                ].properties = defaultEventDisplayProperties();
                                break;
                            }
                            case "SampleDisplayProperties": {
                                appState.local.displays[
                                    activeDisplayId - 1
                                ].properties = defaultSampleDisplayProperties(
                                    newSource.stream,
                                );
                                break;
                            }
                            default: {
                                throw new Error("unsupported propertiesType");
                            }
                        }
                    }
                    display.target = sources[newSelectedIndex].target;
                }}
            ></Dropdown>
            {#if display.properties.type === "EventDisplayProperties"}
                <Dropdown
                    choices={["Exponential", "Linear", "Window"]}
                    bind:selectedIndex={display.properties.style}
                ></Dropdown>
                <Dropdown
                    choices={namedColormaps.map(colormap => colormap.name)}
                    bind:selectedIndex={display.properties.colormapIndex}
                ></Dropdown>
            {:else if display.properties.type === "SampleDisplayProperties"}
                {#each display.properties.namesAndRangesIndices as nameAndRangeIndex, index}
                    <div style="color: #FFFFFF">{nameAndRangeIndex[0]}</div>
                    <Dropdown
                        choices={constants.CHART_RANGES.map(range =>
                            range === 0.0 ? "Hidden" : `${range} s`,
                        )}
                        selectedIndex={nameAndRangeIndex[1]}
                        onChange={selectedIndex => {
                            if (
                                display.properties.type ===
                                "SampleDisplayProperties"
                            ) {
                                display.properties.namesAndRangesIndices[
                                    index
                                ][1] = selectedIndex;
                                display.properties.hash =
                                    utilities.nextUnique();
                            }
                        }}
                    ></Dropdown>
                {/each}
            {/if}
        {/if}
    </div>
</div>

<style>
    .display-pane {
        width: 0;
        border-top: 1px solid var(--background-0);
        height: calc(100vh - var(--status-bar-height));
        transition: width 0.3s;
        overflow: hidden;
        position: relative;
    }

    .display-pane.open {
        width: var(--display-pane-width);
    }

    .content {
        width: var(--display-pane-width);
        height: calc(100vh - 1px - var(--status-bar-height));
        background-color: var(--background-2);
        position: absolute;
        left: 0;
        top: 0;
        padding: 20px;
    }

    .message {
        color: #aaaaaa;
    }
</style>
