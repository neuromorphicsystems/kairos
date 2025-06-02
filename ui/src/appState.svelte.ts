import type { Layout } from "./constants";

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

interface SharedState {
    devices: {
        id: number;
        name: string;
        serial: string;
        streams: Stream[];
    }[];
}

export interface EventDisplayProperties {
    type: "EventDisplayProperties";
    colormapIndex: number;
    style: number;
    tau: number;
}

export function defaultEventDisplayProperties(): EventDisplayProperties {
    return {
        type: "EventDisplayProperties",
        colormapIndex: 0,
        style: 0,
        tau: 500000.0,
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
                ["Temperature", 3],
                ["External events", 3],
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
    displays: [Display, Display, Display, Display];
}

interface AppState {
    shared: SharedState;
    local: LocalState;
}

const appState: AppState = $state({
    shared: {
        devices: [],
    },
    local: {
        connectionStatus: "connecting",
        layout: "h",
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
    },
});

export default appState;
