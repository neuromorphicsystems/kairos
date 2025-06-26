import type {
    Autostop,
    Autotrigger,
    EventDisplayProperties,
    Lookback,
    RecordState,
    SampleDisplayProperties,
} from "./appState.svelte";
import type { Configuration } from "./deviceConfiguration";

import { toast } from "svelte-sonner";

import * as constants from "./constants";
import * as utilities from "./utilities";
import appState, { defaultNamesAndRanges } from "./appState.svelte";
import EventPainter from "./eventPainter";
import Evk4SamplesDecoder from "./evk4SamplesDecoder";
import SamplePainter from "./samplePainter";

// @ts-ignore
import transportWorkerUrl from "./transportWorker.wts";
// @ts-ignore
import evt3DecoderWorkerUrl from "./evt3DecoderWorker.wts";

type Decoder = Worker | Evk4SamplesDecoder;

type Painter = EventPainter | SamplePainter;

const transportWorker = new Worker(transportWorkerUrl, {
    type: "module",
    name: "transport-worker",
});

const sourceIdToDecoderAndPainter: Map<number, [Decoder, Painter]> = new Map();

const decoder = new TextDecoder();

transportWorker.addEventListener("message", ({ data }) => {
    switch (data.type) {
        case constants.TRANSPORT_TO_MAIN_CONNECTION_STATUS: {
            appState.local.connectionStatus = data.status;
            break;
        }
        case constants.TRANSPORT_TO_MAIN_MESSAGE_BUFFER: {
            const size = new Uint32Array(data.buffer, 0, 1)[0];
            const message = JSON.parse(
                decoder.decode(new Uint8Array(data.buffer, 4, size - 4)),
            );
            transportWorker.postMessage(
                {
                    type: constants.MAIN_TO_TRANSPORT_BUFFER,
                    streamId: data.streamId,
                    buffer: data.buffer,
                },
                {
                    transfer: [data.buffer],
                },
            );

            switch (message.type) {
                case "SharedClientState": {
                    appState.shared = message.content;
                    for (
                        let index = appState.local.nextErrorIndex;
                        index < appState.shared.errors.length;
                        ++index
                    ) {
                        toast.error(appState.shared.errors[index], {
                            duration: Number.POSITIVE_INFINITY,
                        });
                    }
                    appState.local.nextErrorIndex =
                        appState.shared.errors.length;
                    if (appState.shared.devices.length === 0) {
                        appState.local.deviceIndex = null;
                    } else if (
                        appState.local.deviceIndex == null ||
                        appState.local.deviceIndex >=
                            appState.shared.devices.length
                    ) {
                        appState.local.deviceIndex = 0;
                    }
                    for (const display of appState.local.displays) {
                        if (display != null) {
                            if (display.target != null) {
                                let found = false;
                                for (const device of appState.shared.devices) {
                                    if (
                                        device.id === display.target.deviceId &&
                                        device.streams.length >
                                            display.target.streamIndex
                                    ) {
                                        found = true;
                                        break;
                                    }
                                }
                                if (!found) {
                                    display.target = null;
                                    if (
                                        display.properties.type ===
                                        "SampleDisplayProperties"
                                    ) {
                                        display.properties.namesAndRangesIndices =
                                            [];
                                        display.properties.hash =
                                            utilities.nextUnique();
                                    }
                                }
                            }
                            if (display.target == null) {
                                for (const device of appState.shared.devices) {
                                    let streamIndex = 0;
                                    for (const stream of device.streams) {
                                        if (
                                            (display.properties.type ===
                                                "EventDisplayProperties" &&
                                                stream.type === "Evt3") ||
                                            (display.properties.type ===
                                                "SampleDisplayProperties" &&
                                                stream.type === "Evk4Samples")
                                        ) {
                                            display.target = {
                                                deviceId: device.id,
                                                streamIndex,
                                            };
                                            if (
                                                display.properties.type ===
                                                "SampleDisplayProperties"
                                            ) {
                                                display.properties.namesAndRangesIndices =
                                                    defaultNamesAndRanges(
                                                        stream,
                                                    );
                                                display.properties.hash =
                                                    utilities.nextUnique();
                                            }
                                            break;
                                        }
                                        ++streamIndex;
                                    }
                                    if (display.target != null) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
                case "SharedRecordingsState": {
                    appState.sharedRecordings = message.content;
                    break;
                }
                default: {
                    throw new Error(`unsupported message type ${message.type}`);
                }
            }
            break;
        }
        case constants.TRANSPORT_TO_MAIN_RECORD_STATE_BUFFER: {
            const dataView = new DataView(data.buffer);
            const size = dataView.getUint32(0, true);
            const deviceId = dataView.getUint32(4, true);
            const recordState: RecordState = {
                lookback:
                    dataView.getUint8(8) === 0
                        ? null
                        : {
                              duration_us: dataView.getBigUint64(9, true),
                              size_bytes: dataView.getBigUint64(17, true),
                          },
                recording:
                    dataView.getUint8(25) === 0
                        ? null
                        : {
                              name: decoder.decode(
                                  new Uint8Array(data.buffer, 42, size - 42),
                              ),
                              duration_us: dataView.getBigUint64(26, true),
                              size_bytes: dataView.getBigUint64(34, true),
                          },
            };
            transportWorker.postMessage(
                {
                    type: constants.MAIN_TO_TRANSPORT_BUFFER,
                    streamId: data.streamId,
                    buffer: data.buffer,
                },
                {
                    transfer: [data.buffer],
                },
            );
            appState.deviceIdToRecordState[deviceId] = recordState;
            break;
        }
        case constants.TRANSPORT_TO_DECODE_BUFFER: {
            const decoderAndPainter = sourceIdToDecoderAndPainter.get(
                data.sourceId,
            );
            if (decoderAndPainter == null) {
                throw new Error(
                    `no decoder worker for source ${data.sourceId} (stream ${data.streamId})`,
                );
            }
            decoderAndPainter[0].postMessage(
                {
                    type: data.type,
                    streamId: data.streamId,
                    sourceId: data.sourceId,
                    buffer: data.buffer,
                },
                {
                    transfer: [data.buffer],
                },
            );
            break;
        }
        default: {
            console.error(
                `unexpected message from transport ${JSON.stringify(data)}`,
            );
        }
    }
});

let protocol = window.location.protocol;
let hostname = window.location.hostname;
let port = window.location.port === "" ? "" : `:${window.location.port}`;
let path = window.location.pathname;
if (window.location.protocol === "file:") {
    protocol = "http:";
    hostname = "localhost";
    port = ":3000";
    path = "/";
} else if (!path.endsWith("/")) {
    path = `${path}/`;
}
transportWorker.postMessage({
    type: constants.MAIN_TO_TRANSPORT_SETUP,
    protocol,
    hostname,
    port,
    path,
});

function sendMessageToServer(message: any) {
    const encoder = new TextEncoder();
    const buffer = encoder.encode(JSON.stringify(message));
    const bufferWithSize = new ArrayBuffer(buffer.length + 4);
    new Uint32Array(bufferWithSize, 0, 1)[0] = bufferWithSize.byteLength;
    new Uint8Array(bufferWithSize, 4, buffer.length).set(buffer, 0);
    transportWorker.postMessage(
        {
            type: constants.MAIN_TO_TRANSPORT_MESSAGE,
            buffer: bufferWithSize,
        },
        { transfer: [bufferWithSize] },
    );
}

function deviceIdAndStreamIndexToSourceId(
    deviceId: number,
    streamIndex: number,
): number {
    return ((deviceId << 8) & 0xffffff) | (streamIndex & 0xff);
}

function startStream(
    deviceId: number,
    streamIndex: number,
): [Decoder, Painter] {
    const sourceId = deviceIdAndStreamIndexToSourceId(deviceId, streamIndex);
    let stream = null;
    for (const device of appState.shared.devices) {
        if (device.id === deviceId) {
            if (streamIndex >= device.streams.length) {
                throw new Error(
                    `device ${device.id} has only ${device.streams.length} streams (requested index ${streamIndex})`,
                );
            }
            stream = $state.snapshot(device.streams[streamIndex]);
            break;
        }
    }
    if (stream == null) {
        throw new Error(`device ${deviceId} not found`);
    }
    const decoderAndPainter = sourceIdToDecoderAndPainter.get(sourceId);
    if (decoderAndPainter != null) {
        return decoderAndPainter;
    }
    if (stream.type === "Evt3") {
        const decoder = new Worker(evt3DecoderWorkerUrl, {
            type: "module",
            name: `decoder-worker-source-${stream.sourceId}`,
        });
        const painter = new EventPainter(decoder, stream.width, stream.height);
        sourceIdToDecoderAndPainter.set(sourceId, [decoder, painter]);
        decoder.addEventListener("message", ({ data }) => {
            switch (data.type) {
                case constants.DECODE_TO_MAIN_READY: {
                    decoder.postMessage({
                        type: constants.MAIN_TO_DECODE_SETUP,
                        stream,
                    });
                    sendMessageToServer({
                        type: "StartStream",
                        stream_id: sourceId,
                    });
                    break;
                }
                case constants.DECODE_TO_TRANSPORT_BUFFER: {
                    transportWorker.postMessage(
                        {
                            type: data.type,
                            streamId: data.streamId,
                            sourceId: data.sourceId,
                            buffer: data.buffer,
                        },
                        {
                            transfer: [data.buffer],
                        },
                    );
                    break;
                }
                case constants.DECODE_TO_PAINT_BUFFER: {
                    (painter as EventPainter).handleBuffer(
                        data.buffer,
                        data.glCurrentT,
                        data.displayT,
                    );
                    break;
                }
                default: {
                    console.error(
                        `unexpected message from decoder ${JSON.stringify(data)}`,
                    );
                }
            }
        });
        return [decoder, painter];
    } else if (stream.type === "Evk4Samples") {
        const painter = new SamplePainter([
            {
                name: "Event rate",
                yAxes: [
                    {
                        label: "Event rate (Hz)",
                        units: "Hz",
                        logarithmic: true,
                    },
                    null,
                ],
                curves: [
                    {
                        name: "Total",
                        color: "#5C538B",
                        dash: null,
                        axis: 0,
                        order: 2,
                    },
                    {
                        name: "On",
                        color: "#4F88B9",
                        dash: null,
                        axis: 0,
                        order: 1,
                    },
                    {
                        name: "Off",
                        color: "#723959",
                        dash: null,
                        axis: 0,
                        order: 0,
                    },
                ],
            },
            {
                name: "Illuminance",
                yAxes: [
                    {
                        label: "Illuminance (lx)",
                        units: "lx",
                        logarithmic: false,
                    },
                    null,
                ],
                curves: [
                    {
                        name: "Illuminance",
                        color: "#C3A34B",
                        dash: null,
                        axis: 0,
                        order: 0,
                    },
                ],
            },
            {
                name: "Temperature",
                yAxes: [
                    {
                        label: "Temperature (ºC)",
                        units: "ºC",
                        logarithmic: false,
                    },
                    null,
                ],
                curves: [
                    {
                        name: "Temperature",
                        color: "#874037",
                        dash: null,
                        axis: 0,
                        order: 0,
                    },
                ],
            },
            {
                name: "External events",
                yAxes: [
                    {
                        label: "External events",
                        units: null,
                        logarithmic: false,
                    },
                    null,
                ],
                curves: [
                    {
                        name: "Rising",
                        color: "#B4DEC6",
                        dash: null,
                        axis: 0,
                        order: 1,
                    },
                    {
                        name: "Falling",
                        color: "#74BBCD",
                        dash: null,
                        axis: 0,
                        order: 0,
                    },
                ],
            },
            {
                name: "Autotrigger",
                yAxes: [
                    {
                        label: "Event rate (Hz)",
                        units: "Hz",
                        logarithmic: true,
                    },
                    {
                        label: "Ratio",
                        units: null,
                        logarithmic: true,
                    },
                ],
                curves: [
                    {
                        name: "Short",
                        color: "#5C538B",
                        dash: null,
                        axis: 0,
                        order: 3,
                    },
                    {
                        name: "Long",
                        color: "#723959",
                        dash: null,
                        axis: 0,
                        order: 2,
                    },
                    {
                        name: "Ratio",
                        color: "#AAAAAA",
                        dash: null,
                        axis: 1,
                        order: 1,
                    },
                    {
                        name: "Threshold",
                        color: "#AAAAAA",
                        dash: [5, 5],
                        axis: 1,
                        order: 0,
                    },
                ],
            },
        ]);
        const decoder = new Evk4SamplesDecoder(
            painter,
            (streamId, sourceId, buffer) => {
                transportWorker.postMessage(
                    {
                        type: constants.DECODE_TO_TRANSPORT_BUFFER,
                        streamId,
                        sourceId,
                        buffer,
                    },
                    {
                        transfer: [buffer],
                    },
                );
            },
        );
        sourceIdToDecoderAndPainter.set(sourceId, [decoder, painter]);
        sendMessageToServer({
            type: "StartStream",
            stream_id: sourceId,
        });
        return [decoder, painter];
    } else {
        throw new Error(`stream type ${stream.type} not implemented`);
    }
}

export function stopStream(deviceId: number, streamIndex: number) {
    // @DEV this should be a special constants.MAIN_TO... message
}

export function startRecording(deviceId: number, name: string) {
    sendMessageToServer({
        type: "StartRecording",
        device_id: deviceId,
        name,
    });
}

export function stopRecording(deviceId: number) {
    sendMessageToServer({
        type: "StopRecording",
        device_id: deviceId,
    });
}

export function updateConfiguration(
    deviceId: number,
    configuration: Configuration,
) {
    sendMessageToServer({
        type: "UpdateConfiguration",
        device_id: deviceId,
        configuration,
    });
}

export function updateLookback(deviceId: number, lookback: Lookback) {
    sendMessageToServer({
        type: "UpdateLookback",
        device_id: deviceId,
        lookback,
    });
}

export function updateAutostop(deviceId: number, autostop: Autostop) {
    sendMessageToServer({
        type: "UpdateAutostop",
        device_id: deviceId,
        autostop,
    });
}

export function updateAutotrigger(deviceId: number, autotrigger: Autotrigger) {
    sendMessageToServer({
        type: "UpdateAutotrigger",
        device_id: deviceId,
        autotrigger,
    });
}

export function convert(names: string[]) {
    sendMessageToServer({
        type: "Convert",
        names,
    });
}

export function cancelConvert() {
    sendMessageToServer({
        type: "CancelConvert",
    });
}

export function attach(
    deviceId: number,
    streamIndex: number,
    element: HTMLElement,
    overlay: HTMLElement,
    properties: EventDisplayProperties | SampleDisplayProperties,
): number {
    const [_, painter] = startStream(deviceId, streamIndex);
    if (painter.type === "EventPainter") {
        if (properties.type !== "EventDisplayProperties") {
            throw new Error(
                `${properties.type} used to attach a canvas to a painter with type ${painter.type}`,
            );
        }
        return painter.attach(
            element as HTMLCanvasElement,
            overlay,
            properties,
        );
    } else if (painter.type === "SamplePainter") {
        if (properties.type !== "SampleDisplayProperties") {
            throw new Error(
                `${properties.type} used to attach a canvas to a painter with type ${painter.type}`,
            );
        }
        return painter.attach(element, properties);
    }
    throw new Error(`unsupported painter type`);
}

export function detach(
    deviceId: number,
    streamIndex: number,
    elementId: number,
) {
    const decoderAndPainter = sourceIdToDecoderAndPainter.get(
        deviceIdAndStreamIndexToSourceId(deviceId, streamIndex),
    );
    if (decoderAndPainter != null) {
        decoderAndPainter[1].detach(elementId);
    }
}
