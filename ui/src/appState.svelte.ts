import type { Layout } from "./constants";
import type { Configuration } from "./deviceConfiguration";

import * as utilities from "./utilities";
import * as constants from "./constants";

interface Evt3Stream {
    type: "Evt3";
    width: number;
    height: number;
}

interface Evk4SamplesStream {
    type: "Evk4Samples";
}

export type Stream = Evt3Stream | Evk4SamplesStream;

interface Device {
    id: number;
    name: string;
    serial: string;
    speed: string;
    bus_number: number;
    address: number;
    streams: Stream[];
    configuration: Configuration;
}

interface SharedState {
    data_directory: string;
    disk_available_and_total_space: [number, number] | null;
    devices: Device[];
    errors: string[];
}

export interface EventDisplayProperties {
    type: "EventDisplayProperties";
    colormapIndex: number;
    style: number;
    tau: number;
    timestamp: boolean;
    reticle: boolean;
    width: number;
    height: number;
    targetX: number;
    targetY: number;
    scale: number;
}

export function defaultEventDisplayProperties(): EventDisplayProperties {
    return {
        type: "EventDisplayProperties",
        colormapIndex: 0,
        style: 0,
        tau: 200000.0,
        timestamp: false,
        reticle: false,
        width: 1280,
        height: 720,
        targetX: 0.5,
        targetY: 0.5,
        scale: 1,
    };
}

export interface SampleDisplayProperties {
    type: "SampleDisplayProperties";
    orientation: "Auto" | "Row" | "Column";
    namesAndRangesIndices: [string, number][];
    hash: number;
}

export function defaultNamesAndRanges(stream: Stream): [string, number][] {
    if (stream == null) {
        return [];
    }
    switch (stream.type) {
        case "Evk4Samples": {
            return [
                ["Event rate", 3],
                ["Illuminance", 3],
                ["Temperature", 0],
                ["External events", 0],
            ];
        }
        default:
            throw new Error(
                `unsupported stream type ${stream.type} in defaultNamesAndRanges`,
            );
    }
}

export function defaultSampleDisplayProperties(
    stream: Stream,
): SampleDisplayProperties {
    return {
        type: "SampleDisplayProperties",
        orientation: "Auto",
        namesAndRangesIndices: defaultNamesAndRanges(stream),
        hash: utilities.nextUnique(),
    };
}

interface Display {
    target: {
        deviceId: number;
        streamIndex: number;
    };
    properties: EventDisplayProperties | SampleDisplayProperties;
}

interface LocalState {
    connectionStatus: constants.ConnectionStatus;
    layout: Layout;
    deviceIndex: number | null;
    displays: [Display, Display, Display, Display];
    nextErrorIndex: number;
}

interface RecordState {}

interface AppState {
    shared: SharedState;
    local: LocalState;
    deviceIdToRecordState: { [key: number]: RecordState };
}

const appState: AppState = $state({
    shared: {
        data_directory: "",
        disk_available_and_total_space: null,
        devices: [],
        errors: [],
    },
    local: {
        connectionStatus: "connecting",
        layout: "h",
        deviceIndex: null,
        displays: [
            {
                target: null,
                properties: defaultEventDisplayProperties(),
            },
            {
                target: null,
                properties: defaultSampleDisplayProperties(null),
            },
            null,
            null,
        ],
        nextErrorIndex: 0,
    },
    deviceIdToRecordState: {},
});

export default appState;
