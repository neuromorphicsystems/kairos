import * as constants from "./constants";
import { Painter } from "./painter";

// @ts-ignore
import transportWorkerUrl from "./transportWorker.wts";
// @ts-ignore
import decodeWorkerUrl from "./decodeWorker.wts";

const transportWorker = new Worker(transportWorkerUrl, { type: "module" });

const sourceIdToDecodeWorkerAndPainter: Map<number, [Worker, Painter]> =
    new Map();

let testStarted = false; // @DEV

transportWorker.addEventListener("message", ({ data }) => {
    switch (data.type) {
        case constants.TRANSPORT_TO_MAIN_CONNECTION_STATUS: {
            appState.local.connectionStatus = data.status;
            break;
        }
        case constants.TRANSPORT_TO_MAIN_BUFFER: {
            const size = new Uint32Array(data.buffer, 0, 1)[0];
            const decoder = new TextDecoder();
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

            // @DEV {
            console.log(JSON.stringify(appState));
            if (!testStarted && appState.shared.devices.length > 0) {
                console.log("start stream request"); // @DEV
                startStream(appState.shared.devices[0].id, 0);
            }
            // }
            break;
        }
        case constants.TRANSPORT_TO_DECODE_BUFFER: {
            const decodeWorkerAndPainter = sourceIdToDecodeWorkerAndPainter.get(
                data.sourceId,
            );
            if (decodeWorkerAndPainter == null) {
                throw new Error(
                    `no decode worker for source ${data.sourceId} (stream ${data.streamId})`,
                );
            }
            decodeWorkerAndPainter[0].postMessage(
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

interface SharedState {
    devices: {
        id: number;
        name: string;
        serial: string;
        streams: (
            | {
                  type: "Evt3";
                  width: number;
                  height: number;
              }
            | {
                  type: "Evk4Samples";
              }
        )[];
    }[];
}

interface LocalState {
    connectionStatus: constants.ConnectionStatus;
}

interface AppState {
    shared: SharedState;
    local: LocalState;
}

export const appState: AppState = $state({
    shared: {
        devices: [],
    },
    local: {
        connectionStatus: "connecting",
    },
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

function startStream(deviceId: number, streamIndex: number): [Worker, Painter] {
    const sourceId = deviceIdAndStreamIndexToSourceId(deviceId, streamIndex);
    let stream = null;

    console.log(`startStream(${deviceId}, ${streamIndex})`); // @DEV

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
    const decodeWorkerAndPainter =
        sourceIdToDecodeWorkerAndPainter.get(sourceId);
    if (decodeWorkerAndPainter != null) {
        return decodeWorkerAndPainter;
    }
    const decodeWorker = new Worker(decodeWorkerUrl, { type: "module" });
    let painter: Painter;
    if (stream.type === "Evt3") {
        painter = new Painter(decodeWorker, stream.width, stream.height);
    } else {
        throw new Error(`stream type ${stream.type} not implemented`);
    }
    sourceIdToDecodeWorkerAndPainter.set(sourceId, [decodeWorker, painter]);
    decodeWorker.addEventListener("message", ({ data }) => {
        switch (data.type) {
            case constants.DECODE_TO_MAIN_READY: {
                decodeWorker.postMessage({
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
                painter.handleBuffer(
                    data.buffer,
                    data.currentT,
                    data.glCurrentT,
                );
                console.log(`drawable buffer from ${sourceId}`, data); // @DEV
                break;
            }
            default: {
                console.error(
                    `unexpected message from decode ${JSON.stringify(data)}`,
                );
            }
        }
    });
    return [decodeWorker, painter];
}

export function stopStream(deviceId: number, streamIndex: number) {
    // @DEV this should be a special constants.MAIN_TO... message
}

export function attachCanvas(
    deviceId: number,
    streamIndex: number,
    canvas: HTMLCanvasElement,
): number {
    const [_, painter] = startStream(deviceId, streamIndex);
    return painter.attachCanvas(canvas);
}

export function detachCanvas(
    deviceId: number,
    streamIndex: number,
    canvasId: number,
) {
    const decodeWorkerAndPainter = sourceIdToDecodeWorkerAndPainter.get(
        deviceIdAndStreamIndexToSourceId(deviceId, streamIndex),
    );
    if (decodeWorkerAndPainter != null) {
        decodeWorkerAndPainter[1].detachCanvas(canvasId);
    }
}
