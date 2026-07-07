<script lang="ts">
    import type { ContainerId } from "./constants";
    import type {
        EventDisplayProperties,
        SampleDisplayProperties,
        Stream,
    } from "./appState.svelte";

    import appState, {
        defaultEventDisplayProperties,
        defaultSampleDisplayProperties,
        defaultNamesAndRanges,
    } from "./appState.svelte";
    import * as constants from "./constants";
    import * as utilities from "./utilities";
    import { namedColormaps, rgbaToHex } from "./colormaps";
    import Button from "./button.svelte";
    import Dropdown from "./dropdown.svelte";
    import NumberInput from "./numberInput.svelte";
    import NumberInputWithSlider from "./numberInputWithSlider.svelte";
    import Switch from "./switch.svelte";

    const scaleSliderTicks = [1];
    for (let scale = 10; scale <= constants.MAXIMUM_SCALE; scale += 10) {
        scaleSliderTicks.push(scale);
    }

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
                    label: `${stream.type === "Evt3" ? "Events" : "Samples"} – ${device.serial} (${device.name})`,
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
                label: "Events – Any source",
                stream: null,
                propertiesType: "EventDisplayProperties",
                target: null,
            });
            ++index;
            if (display.properties.type === "SampleDisplayProperties") {
                selectedIndex = index;
            }
            sources.push({
                label: "Samples – Any source",
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
    const colorToGradient: { [key: string]: { on: string; off: string } } =
        $derived(
            Object.fromEntries(
                namedColormaps.map(namedColormap => [
                    namedColormap.name,
                    {
                        on: namedColormap.on.map(rgbaToHex).join(","),
                        off: namedColormap.off.map(rgbaToHex).join(","),
                    },
                ]),
            ),
        );
</script>

<div class="display-pane {open ? 'open' : ''}">
    <div class="content">
        {#if display == null}
            <div class="message">
                Select a layout item to edit its contents and style.
            </div>
        {:else}
            <div class="label">Source</div>
            <Dropdown
                choices={sources.map(source => [source.label, null])}
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
                <div class="label">Decay</div>
                <Dropdown
                    choices={[
                        ["Exponential", exponentialIcon],
                        ["Linear", linearIcon],
                        ["Window", windowIcon],
                    ]}
                    iconStyle="width: 24px; height: 12px; margin-right: 10px; flex-grow: 0; flex-shrink: 0; align-self: baseline;"
                    bind:selectedIndex={display.properties.style}
                ></Dropdown>
                <div class="label">Colormap</div>
                <Dropdown
                    choices={namedColormaps.map(colormap => [
                        colormap.name,
                        colormapIcon,
                    ])}
                    iconStyle="width: 80px; margin-right: 10px; flex-grow: 0; flex-shrink: 0; display: flex; flex-direction: column;"
                    bind:selectedIndex={display.properties.colormapIndex}
                ></Dropdown>
                <Switch
                    label={"Timestamp"}
                    bind:checked={display.properties.timestamp}
                    labelWidth={71}
                ></Switch>
                <Switch
                    label={"Reticle"}
                    bind:checked={display.properties.reticle}
                    labelWidth={71}
                ></Switch>
                <NumberInputWithSlider
                    label={"&#964;"}
                    description={"Decay time constant in milliseconds."}
                    rightSide={true}
                    value={Math.round(display.properties.tau / 1000)}
                    units="ms"
                    step={1}
                    min={1}
                    max={400000}
                    labelWidth={20}
                    labelBold={false}
                    inputWidth={100}
                    digits={0}
                    paddingTop={20}
                    onChange={newTau => {
                        (display.properties as EventDisplayProperties).tau =
                            newTau * 1000;
                    }}
                    sliderLogarithmic={true}
                    sliderTicks={[2, 20, 200, 2000, 20000, 200000]}
                    sliderWidth={260}
                />
                <div style="display: flex; justify-content: space-between">
                    <NumberInput
                        label={"X"}
                        description={"Center of the field of view in pixel coordinates (from the left, starting at 0). If the field of view is exactly centered on a pixel (x, y), this field contains x + 0.5."}
                        rightSide={true}
                        value={(display.properties as EventDisplayProperties)
                            .targetX *
                            (display.properties as EventDisplayProperties)
                                .width}
                        units={null}
                        step={"any"}
                        min={null}
                        max={null}
                        labelWidth={20}
                        labelBold={false}
                        inputWidth={100}
                        digits={3}
                        paddingTop={20}
                        onChange={newX => {
                            (
                                display.properties as EventDisplayProperties
                            ).targetX =
                                newX /
                                (display.properties as EventDisplayProperties)
                                    .width;
                        }}
                    />
                    <NumberInput
                        label={"Y"}
                        description={"Center of the field of view in pixel coordinates (from the top, starting at 0). If the field of view is exactly centered on a pixel (x, y), this field contains y + 0.5."}
                        rightSide={true}
                        value={(display.properties as EventDisplayProperties)
                            .targetY *
                            (display.properties as EventDisplayProperties)
                                .height}
                        units={null}
                        step={"any"}
                        min={null}
                        max={null}
                        labelWidth={20}
                        labelBold={false}
                        inputWidth={100}
                        digits={3}
                        paddingTop={20}
                        onChange={newY => {
                            (
                                display.properties as EventDisplayProperties
                            ).targetY =
                                newY /
                                (display.properties as EventDisplayProperties)
                                    .height;
                        }}
                    />
                </div>
                <NumberInputWithSlider
                    label={"S"}
                    description={"Display zoom level. The actual scale also depends on the size of the display area in the layout."}
                    rightSide={true}
                    bind:value={display.properties.scale}
                    units={null}
                    step={"any"}
                    min={1}
                    max={constants.MAXIMUM_SCALE}
                    labelWidth={20}
                    labelBold={false}
                    inputWidth={100}
                    digits={3}
                    paddingTop={20}
                    sliderLogarithmic={false}
                    sliderTicks={scaleSliderTicks}
                    sliderWidth={260}
                />
                <Button
                    label="Reset position"
                    icon={resetIcon}
                    onClick={() => {
                        (display.properties as EventDisplayProperties).targetX =
                            0.5;
                        (display.properties as EventDisplayProperties).targetY =
                            0.5;
                        (display.properties as EventDisplayProperties).scale =
                            1.0;
                    }}
                ></Button>
            {:else if display.properties.type === "SampleDisplayProperties"}
                <div class="label">Orientation</div>
                <Dropdown
                    choices={[
                        ["Auto", null],
                        ["Row", null],
                        ["Column", null],
                    ]}
                    selectedIndex={{ Auto: 0, Row: 1, Column: 2 }[
                        display.properties.orientation
                    ]}
                    onChange={selectedIndex => {
                        switch (selectedIndex) {
                            case 0:
                                (
                                    display.properties as SampleDisplayProperties
                                ).orientation = "Auto";
                                break;
                            case 1:
                                (
                                    display.properties as SampleDisplayProperties
                                ).orientation = "Row";
                                break;

                            case 2:
                                (
                                    display.properties as SampleDisplayProperties
                                ).orientation = "Column";
                                break;
                            default:
                                throw new Error(
                                    `unexpected index ${selectedIndex}`,
                                );
                        }
                    }}
                ></Dropdown>

                {#each display.properties.namesAndRangesIndices as nameAndRangeIndex, index}
                    <div class="label">{nameAndRangeIndex[0]}</div>
                    <Dropdown
                        choices={constants.CHART_RANGES.map(range => [
                            range === 0.0 ? "Hidden" : `${range} s`,
                            null,
                        ])}
                        selectedIndex={nameAndRangeIndex[1]}
                        onChange={selectedIndex => {
                            (
                                display.properties as SampleDisplayProperties
                            ).namesAndRangesIndices[index][1] = selectedIndex;
                            (
                                display.properties as SampleDisplayProperties
                            ).hash = utilities.nextUnique();
                        }}
                    ></Dropdown>
                {/each}
            {/if}
        {/if}
    </div>
</div>

{#snippet resetIcon()}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"
        ><path
            fill="#DDDDDD"
            fill-rule="evenodd"
            d="M53.0565026,1.48544435 C54.9975028,3.44959446 54.9785028,6.61534464 53.0145026,8.55639476 L45.6015022,15.8824952 L49.5,15.8824952 C72.9160038,15.8824952 92,34.6569963 92,57.9409976 C92,81.2249989 72.9160038,100 49.5,100 C26.0840011,100 7,81.2249989 7,57.9409976 C7,55.1799974 9.23860013,52.9409973 12,52.9409973 C14.7615004,52.9409973 17,55.1799974 17,57.9409976 C17,75.5909986 31.4950014,90 49.5,90 C67.5050035,90 82,75.5909986 82,57.9409976 C82,40.2909966 67.5050035,25.8824957 49.5,25.8824957 L45.6015022,25.8824957 L53.0145026,33.2084962 C54.9785028,35.1494963 54.9975028,38.3149965 53.0565026,40.2794966 C51.1155025,42.2434967 47.9495023,42.2619967 45.9855022,40.3209966 L29.9140013,24.4384957 C28.9635013,23.4994956 28.4285012,22.2184955 28.4285012,20.8824955 C28.4285012,19.5459954 28.9635013,18.2654953 29.9140013,17.3259953 L45.9855022,1.44359435 C47.9495023,-0.497405763 51.1155025,-0.478705762 53.0565026,1.48544435 Z"
        /></svg
    >
{/snippet}

{#snippet exponentialIcon(choice)}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"
        ><path
            fill="#DDDDDD"
            fill-rule="evenodd"
            d="M4.54747351e-13,0 C4.54747351e-13,23.8440429 4.54747351e-13,57.1773762 4.54747351e-13,100 L200,100 C106.683431,95.5572625 40.0167644,62.2239291 4.54747351e-13,0 Z"
        /></svg
    >
{/snippet}

{#snippet linearIcon(choice)}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"
        ><polygon
            fill="#DDDDDD"
            fill-rule="evenodd"
            points="0 100 0 0 200 100"
        /></svg
    >
{/snippet}

{#snippet windowIcon(choice)}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"
        ><rect
            width="200"
            height="100"
            fill="#DDDDDD"
            fill-rule="evenodd"
        /></svg
    >
{/snippet}

{#snippet colormapIcon(choice)}
    <div
        style="height: 6px; border-radius: 4px 4px 0 0; background: linear-gradient(90deg,{colorToGradient[
            choice
        ].on})"
    ></div>
    <div
        style="height: 6px; border-radius: 0 0 4px 4px; background: linear-gradient(90deg,{colorToGradient[
            choice
        ].off})"
    ></div>
{/snippet}

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
        overflow-y: auto;
    }

    .label {
        font-size: 14px;
        padding-bottom: 5px;
        color: var(--content-0);
    }

    .label:not(:first-of-type) {
        padding-top: 20px;
    }

    .message {
        font-size: 14px;
        color: var(--content-0);
    }
</style>
