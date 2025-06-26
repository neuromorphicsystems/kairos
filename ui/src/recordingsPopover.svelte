<script lang="ts">
    import type { Recording } from "./appState.svelte";
    import appState from "./appState.svelte";
    import Button from "./button.svelte";
    import PopoverMask from "./popoverMask.svelte";
    import { convert, cancelConvert } from "./protocol.svelte";
    import * as utilities from "./utilities";

    let {
        open,
        left,
        onClose,
    }: {
        open: boolean;
        left: number;
        onClose: () => void;
    } = $props();

    let nameToSelected: { [key: string]: boolean } = $state(
        Object.fromEntries(
            appState.sharedRecordings.recordings.map(recording => [
                recording.name,
                true,
            ]),
        ),
    );
    let selectedCount: number = $state(
        appState.sharedRecordings.recordings.filter(recording =>
            selectable(recording),
        ).length,
    );
    const converting = $derived(
        appState.sharedRecordings.recordings.some(
            recording =>
                recording.state.type === "Queued" ||
                recording.state.type === "Converting",
        ),
    );
    $effect(() => {
        let count = 0;
        for (const recording of appState.sharedRecordings.recordings) {
            if (nameToSelected[recording.name] == null) {
                nameToSelected[recording.name] = true;
            }
            if (selectable(recording) && nameToSelected[recording.name]) {
                ++count;
            }
        }
        selectedCount = count;
    });

    function selectable(recording: Recording): boolean {
        return recording.state.type === "Complete" && !recording.state.zip;
    }
</script>

<PopoverMask bind:open {onClose}></PopoverMask>

<div class="content {open ? '' : 'hidden'}" style="left: {left}px">
    <div class="property">
        <div class="name">Data directory</div>
        <div class="value">
            {appState.sharedRecordings.data_directory}
        </div>
    </div>
    <div class="buttons">
        <div class="left">
            <Button
                label="Select all"
                onClick={() => {
                    for (const name of Object.keys(nameToSelected)) {
                        nameToSelected[name] = true;
                    }
                }}
            ></Button>
            <Button
                label="Unselect all"
                onClick={() => {
                    for (const name of Object.keys(nameToSelected)) {
                        nameToSelected[name] = false;
                    }
                }}
            ></Button>
            {#if !converting}
                <Button
                    disabled={selectedCount === 0}
                    label="Convert selected"
                    onClick={() => {
                        convert(
                            appState.sharedRecordings.recordings
                                .filter(
                                    recording =>
                                        selectable(recording) &&
                                        nameToSelected[recording.name],
                                )
                                .map(recording => recording.name),
                        );
                    }}
                ></Button>
            {/if}
        </div>
        <div class="right">
            {#if converting}
                <Button
                    label="Cancel conversion"
                    onClick={() => {
                        cancelConvert();
                    }}
                ></Button>
            {/if}
        </div>
    </div>
    <div class="selected-label">
        Selected {selectedCount === 0 ? "no" : selectedCount.toString()} recording{selectedCount ===
        1
            ? ""
            : "s"}
    </div>
    <div class="table-wrapper">
        <table>
            <thead>
                <tr>
                    <th>Selected</th>
                    <th>Name</th>
                    <th>Size</th>
                    <th>State</th>
                    <th>Zip</th>
                </tr>
            </thead>
            <tbody>
                {#each appState.sharedRecordings.recordings as recording}
                    <tr
                        class={selectable(recording) ? "selectable" : ""}
                        onclick={() => {
                            if (selectable(recording)) {
                                nameToSelected[recording.name] =
                                    !nameToSelected[recording.name];
                            }
                        }}
                    >
                        <td>
                            {#if selectable(recording)}
                                {#if nameToSelected[recording.name]}
                                    &#10003;
                                {/if}
                            {:else}
                                -
                            {/if}
                        </td>
                        <td>
                            {recording.name}
                        </td>
                        <td>
                            {recording.state.type === "Ongoing"
                                ? "..."
                                : utilities.sizeToString(
                                      BigInt(recording.state.size_bytes),
                                  )}
                        </td>
                        <td>
                            {recording.state.type}
                        </td>
                        <td>
                            {#if recording.state.type !== "Ongoing" && recording.state.type !== "Incomplete" && recording.state.zip}
                                &#10003;
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>

<style>
    .content.hidden {
        display: none;
    }

    .content {
        position: fixed;
        top: calc(var(--status-bar-height) - 4px);
        width: var(--recordings-width);
        max-height: calc(100vh - var(--status-bar-height) + 4px - 20px);
        background-color: var(--background-3);
        border: 1px solid var(--border);
        border-radius: 8px;
        z-index: 11;
        padding: 20px;
        box-shadow: 0 0 8px 0 #00000080;
        display: flex;
        flex-direction: column;
    }

    .property {
        flex-shrink: 0;
        font-size: 14px;
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 5px 10px;
    }

    .property .name {
        flex-grow: 0;
        flex-shrink: 0;
        color: var(--content-0);
    }

    .property .value {
        color: var(--content-2);
        overflow-wrap: break-word;
        max-width: calc(var(--recordings-width) - 40px);
    }

    .selected-label {
        font-size: 14px;
        padding-bottom: 10px;
        color: var(--content-0);
    }

    .buttons {
        display: flex;
        justify-content: space-between;
        padding-bottom: 10px;
    }

    .buttons .left {
        display: flex;
        gap: 10px;
    }

    .buttons .right {
        display: flex;
    }

    .table-wrapper {
        overflow-y: auto;
        border-radius: 8px;
    }

    table {
        flex-shrink: 1;
        font-size: 14px;
        border-collapse: collapse;
        width: 100%;
        white-space: nowrap;
        background-color: var(--background-1);
    }

    thead tr th {
        height: 40px;
        padding: 10px;
        padding-bottom: 11px;
        box-shadow: inset 0 -1px 0 var(--border);
        text-align: left;
        user-select: none;
        -webkit-user-select: none;
        font-size: 12px;
        line-height: 14px;
        color: var(--content-1);
        background-color: var(--background-1);
    }

    td {
        padding: 10px;
    }

    tr:not(:last-of-type) {
        border-bottom: 1px solid var(--border);
    }

    th {
        position: sticky;
        top: 0;
    }

    tbody tr {
        height: 40px;
        color: var(--content-2);
    }

    tbody tr.selectable {
        cursor: pointer;
    }

    tbody tr:hover {
        background-color: var(--background-2);
    }
</style>
