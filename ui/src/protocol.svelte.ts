import type {
    EventDisplayProperties,
    SampleDisplayProperties,
} from "./appState.svelte";

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
        case constants.TRANSPORT_TO_MAIN_BUFFER: {
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
            appState.shared = message;
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
                                display.properties.namesAndRangesIndices = [];
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
                                            defaultNamesAndRanges(stream);
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

    console.log(
        `${utilities.utcString()} | startStream(${deviceId}, ${streamIndex})`,
    ); // @DEV

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
                        id: sourceId,
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
                yLabel: "Event rate (Hz)",
                logarithmic: true,
                curvesNamesAndColors: [
                    ["Total", "#5C538B"],
                    ["On", "#4F88B9"],
                    ["Off", "#723959"],
                ],
            },
            {
                name: "Illuminance",
                yLabel: "Illuminance",
                logarithmic: false,
                curvesNamesAndColors: [["Illuminance", "#C3A34B"]],
            },
            {
                name: "Temperature",
                yLabel: "Temperature (ºC)",
                logarithmic: false,
                curvesNamesAndColors: [["Temperature", "#874037"]],
            },
            {
                name: "External events",
                yLabel: "External events",
                logarithmic: false,
                curvesNamesAndColors: [
                    ["Rising", "#B4DEC6"],
                    ["Falling", "#74BBCD"],
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
            id: sourceId,
        });
        return [decoder, painter];
    } else {
        throw new Error(`stream type ${stream.type} not implemented`);
    }
}

export function stopStream(deviceId: number, streamIndex: number) {
    // @DEV this should be a special constants.MAIN_TO... message
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
