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

export interface Lookback {
    enabled: boolean;
    maximum_duration_us: number;
    maximum_size_bytes: number;
}

export interface Autostop {
    enabled: boolean;
    duration_us: number;
}

export interface Autotrigger {
    enabled: boolean;
    short_sliding_window: number;
    long_sliding_window: number;
    threshold: number;
}

interface Device {
    id: number;
    name: string;
    serial: string;
    speed: string;
    bus_number: number;
    address: number;
    streams: Stream[];
    configuration: Configuration;
    lookback: Lookback;
    autostop: Autostop;
    autotrigger: Autotrigger;
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
                ["Autotrigger", 0],
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
    layoutToPosition: { [key in Layout]: number[] };
    deviceIndex: number | null;
    displays: [Display, Display, Display, Display];
    nextErrorIndex: number;
}

export interface RecordState {
    lookback: {
        duration_us: bigint;
        size_bytes: bigint;
    } | null;
    recording: {
        name: string;
        duration_us: bigint;
        size_bytes: bigint;
    } | null;
}

type RecordingState =
    | {
          type: "Ongoing";
      }
    | {
          type: "Incomplete";
          size_bytes: number;
      }
    | {
          type: "Complete";
          size_bytes: number;
          zip: boolean;
      }
    | {
          type: "Queued";
          size_bytes: number;
          zip: boolean;
      }
    | {
          type: "Converting";
          size_bytes: number;
          zip: boolean;
      };

export interface Recording {
    name: string;
    state: RecordingState;
}

interface AppState {
    shared: SharedState;
    recordings: Recording[];
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
    recordings: [],
    local: {
        connectionStatus: "connecting",
        layout: "h",
        layoutToPosition: {
            full: [],
            h: [0.75],
            hv1: [0.5, 0.5],
            hv2: [0.5, 0.5],
            v: [0.5],
            vh1: [0.5, 0.5],
            vh2: [0.5, 0.5],
            hv1v2: [0.5, 0.5, 0.5],
            vh1h2: [0.5, 0.5, 0.5],
        },
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
