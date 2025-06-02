import * as constants from "./constants";

const idToStream: Map<number, UnidirectionalStream> = new Map();
let nextStreamId: number = 0;
let messageStream: MessageStream = null;
let started = false;

async function readDescription(
    reader: ReadableStreamBYOBReader,
): Promise<[number, number, number]> {
    let buffer = new ArrayBuffer(12);
    let offset = 0;
    while (offset < buffer.byteLength) {
        const { value: view, done } = await reader.read(
            new Uint8Array(buffer, offset, buffer.byteLength - offset),
        );
        if (done) {
            throw new Error("new stream closed before sending an id");
        }
        buffer = view.buffer;
        offset += view.length;
    }
    const [sourceId, recommendedBufferCount, maximumLength] = new Uint32Array(
        buffer,
    );
    return [sourceId, recommendedBufferCount, maximumLength];
}

function unidirectionalStreamOnBuffer(
    stream: UnidirectionalStream,
    buffer: ArrayBuffer,
) {
    postMessage(
        {
            type: constants.TRANSPORT_TO_DECODE_BUFFER,
            streamId: stream.streamId,
            sourceId: stream.sourceId,
            buffer,
        },
        { transfer: [buffer] },
    );
}

class UnidirectionalStream {
    streamId: number;
    sourceId: number;
    buffers: ArrayBuffer[];
    bufferPromiseResolveReject: [
        (buffer: ArrayBuffer) => void,
        (error: any) => void,
    ];
    reader: ReadableStreamBYOBReader;
    onBuffer: (stream: UnidirectionalStream, buffer: ArrayBuffer) => void;
    running: boolean;

    constructor(
        streamId: number,
        sourceId: number,
        bufferCount: number,
        bufferSize: number,
        reader: ReadableStreamBYOBReader,
        onBuffer: (stream: UnidirectionalStream, buffer: ArrayBuffer) => void,
    ) {
        this.streamId = streamId;
        this.sourceId = sourceId;
        this.buffers = new Array(bufferCount)
            .fill(null)
            .map(_ => new ArrayBuffer(bufferSize));
        this.bufferPromiseResolveReject = null;
        this.reader = reader;
        this.onBuffer = onBuffer;
        this.running = true;
        this.spawn();
    }

    async spawn() {
        try {
            let buffer: ArrayBuffer = null;
            let size = 0;
            let offset = 0;
            while (this.running) {
                if (buffer == null) {
                    buffer = await this.nextBuffer();
                }
                if (size === 0 && offset >= 4) {
                    size = new Uint32Array(buffer, 0, 1)[0];
                }
                if (size > 0) {
                    if (offset === size) {
                        this.onBuffer(this, buffer);
                        buffer = null;
                        size = 0;
                        offset = 0;
                        continue;
                    }
                    if (offset > size) {
                        const newBuffer = await this.nextBuffer();
                        new Uint8Array(newBuffer, 0, offset - size).set(
                            new Uint8Array(buffer, size, offset - size),
                            0,
                        );
                        this.onBuffer(this, buffer);
                        offset -= size;
                        size = 0;
                        buffer = newBuffer;
                        continue;
                    }
                }
                const { value: view, done } = await this.reader.read(
                    new Uint8Array(
                        buffer,
                        offset,
                        (size === 0 ? buffer.byteLength : size) - offset,
                    ),
                );
                if (done) {
                    break;
                }
                buffer = view.buffer;
                offset += view.length;
            }
        } catch (error) {
            console.error(
                `streamId=${this.streamId}, sourceId=${this.sourceId}, error: ${error}`,
            );
        }
        if (this.running) {
            this.abort();
        }
    }

    abort() {
        this.running = false;
        if (this.bufferPromiseResolveReject != null) {
            this.bufferPromiseResolveReject[1]("abort");
            this.bufferPromiseResolveReject = null;
        }
        this.reader.cancel();
        console.error(
            `stream ${this.streamId} (source ${this.sourceId}) aborted`,
        );
    }

    async nextBuffer(): Promise<ArrayBuffer> {
        if (this.buffers.length > 0) {
            return this.buffers.shift();
        }
        return new Promise((resolve, reject) => {
            this.bufferPromiseResolveReject = [resolve, reject];
        });
    }
}

class MessageStream extends UnidirectionalStream {
    writer: WritableStreamDefaultWriter;
    pingMessage: Uint8Array;

    constructor(
        streamId: number,
        sourceId: number,
        bufferCount: number,
        bufferSize: number,
        reader: ReadableStreamBYOBReader,
        sender: WritableStream,
        onBuffer: (stream: MessageStream, buffer: ArrayBuffer) => void,
    ) {
        super(streamId, sourceId, bufferCount, bufferSize, reader, onBuffer);
        this.writer = sender.getWriter();
        const encoder = new TextEncoder();
        const buffer = encoder.encode(JSON.stringify({ type: "Ping" }));
        const bufferWithSize = new ArrayBuffer(buffer.length + 4);
        new Uint32Array(bufferWithSize, 0, 1)[0] = bufferWithSize.byteLength;
        new Uint8Array(bufferWithSize, 4, buffer.length).set(buffer, 0);
        this.pingMessage = new Uint8Array(
            bufferWithSize,
            0,
            bufferWithSize.byteLength,
        );
        this.spawnPing();
    }

    abort() {
        super.abort();
        this.writer.abort();
    }

    async spawnPing() {
        try {
            while (this.running) {
                await this.writer.write(this.pingMessage);
                await new Promise(resolve => setTimeout(resolve, 1000));
            }
        } catch (error) {
            console.error(this.streamId, this.sourceId, error);
        }
        if (this.running) {
            this.abort();
        }
    }
}

async function spawnUnidirectionalStreams(
    unidirectionalStreamsReader: ReadableStreamDefaultReader,
) {
    while (true) {
        const { done, value } = await unidirectionalStreamsReader.read();
        if (done) {
            throw new Error("spawnUnidirectionalStreams is done");
        }
        const reader = value.getReader({ mode: "byob" });
        const [sourceId, recommendedBufferCount, maximumLength] =
            await readDescription(reader);
        idToStream.set(
            nextStreamId,
            new UnidirectionalStream(
                nextStreamId,
                sourceId,
                recommendedBufferCount,
                maximumLength,
                reader,
                unidirectionalStreamOnBuffer,
            ),
        );
        ++nextStreamId;
    }
}

async function spawnBidirectionalStreams(
    bidirectionalStreamsReader: ReadableStreamDefaultReader,
) {
    while (true) {
        const { done, value } = await bidirectionalStreamsReader.read();
        if (done) {
            throw new Error("spawnBidirectionalStreams is done");
        }
        const reader = value.readable.getReader({ mode: "byob" });
        const [sourceId, recommendedBufferCount, maximumLength] =
            await readDescription(reader);
        if (sourceId !== constants.MESSAGES_SOURCE_ID) {
            throw new Error(
                `received a request for an unexpected bidirectional stream with id ${sourceId} (expected ${constants.MESSAGES_SOURCE_ID})`,
            );
        }
        if (messageStream != null) {
            throw new Error(
                `received a request for an unexpected messages stream (already exists)`,
            );
        }
        messageStream = new MessageStream(
            sourceId,
            sourceId,
            recommendedBufferCount,
            maximumLength,
            reader,
            value.writable,
            (stream, buffer) => {
                if (
                    new Uint32Array(buffer, 0, 1)[0] === 8 &&
                    new Uint32Array(buffer, 4, 1)[0] === 0x676e6f70 // 'pong'
                ) {
                    if (stream.bufferPromiseResolveReject == null) {
                        stream.buffers.push(buffer);
                    } else {
                        stream.bufferPromiseResolveReject[0](buffer);
                        stream.bufferPromiseResolveReject = null;
                    }
                } else {
                    postMessage(
                        {
                            type: constants.TRANSPORT_TO_MAIN_BUFFER,
                            streamId: stream.streamId,
                            sourceId: stream.sourceId,
                            buffer,
                        },
                        { transfer: [buffer] },
                    );
                }
            },
        );
    }
}

async function connect(
    protocol: string,
    hostname: string,
    port: string,
    path: string,
) {
    let transport: WebTransport = null;
    try {
        const response = await fetch(
            `${protocol}//${hostname}${port}${path}transport-certificate`,
            {
                mode: "cors",
            },
        );
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }
        const endpoint = await response.json();
        transport = new WebTransport(
            `https://${hostname}:${endpoint.port}${path}`,
            {
                allowPooling: false,
                serverCertificateHashes: [
                    {
                        algorithm: "sha-256",
                        value: new Uint8Array(
                            endpoint.hash
                                .split(":")
                                .map(value => parseInt(value, 16)),
                        ),
                    },
                ],
            },
        );
        await transport.ready;
        postMessage({
            type: constants.TRANSPORT_TO_MAIN_CONNECTION_STATUS,
            status: "connected",
        });
        await Promise.all([
            spawnUnidirectionalStreams(
                transport.incomingUnidirectionalStreams.getReader(),
            ),
            spawnBidirectionalStreams(
                transport.incomingBidirectionalStreams.getReader(),
            ),
        ]);
    } catch (error) {
        console.error(error);
    }
    for (const stream of idToStream.values()) {
        stream.abort();
    }
    if (transport != null) {
        transport.close();
    }
    postMessage({
        type: constants.TRANSPORT_TO_MAIN_CONNECTION_STATUS,
        status: "disconnected",
    });
    setTimeout(connect, 1000, protocol, hostname, port, path);
}

self.addEventListener("message", ({ data }) => {
    switch (data.type) {
        case constants.MAIN_TO_TRANSPORT_SETUP: {
            if (started) {
                console.error("transport worker already started");
            } else {
                started = true;
                connect(data.protocol, data.hostname, data.port, data.path);
            }
            break;
        }
        case constants.MAIN_TO_TRANSPORT_BUFFER:
        case constants.DECODE_TO_TRANSPORT_BUFFER: {
            const stream = idToStream.get(data.streamId);
            if (stream != null) {
                if (stream.bufferPromiseResolveReject == null) {
                    stream.buffers.push(data.buffer);
                } else {
                    stream.bufferPromiseResolveReject[0](data.buffer);
                    stream.bufferPromiseResolveReject = null;
                }
            }
            break;
        }
        case constants.MAIN_TO_TRANSPORT_MESSAGE: {
            if (messageStream != null) {
                messageStream.writer.write(
                    new Uint8Array(data.buffer, 0, data.buffer.byteLength),
                );
            }
            break;
        }
        default: {
            console.error(
                `unexpected message in transport ${JSON.stringify(data)}`,
            );
        }
    }
});
