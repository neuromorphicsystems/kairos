import * as constants from "./constants";

class UnidirectionalStream {
    streamId: number;
    sourceId: number;
    buffers: ArrayBuffer[];
    bufferPromiseResolveReject: [
        (buffer: ArrayBuffer) => void,
        (error: any) => void,
    ];
    reader: ReadableStreamBYOBReader;
    buffer: ArrayBuffer;
    size: number;
    offset: number;

    constructor(
        streamId: number,
        sourceId: number,
        bufferCount: number,
        bufferSize: number,
        reader: ReadableStreamBYOBReader,
    ) {
        this.streamId = streamId;
        this.sourceId = sourceId;
        this.buffers = new Array(bufferCount)
            .fill(null)
            .map(_ => new ArrayBuffer(bufferSize));
        this.bufferPromiseResolveReject = null;
        this.reader = reader;
        this.buffer = null;
        this.size = 0;
        this.offset = 0;
    }

    async nextWithBuffer(): Promise<[number, [boolean, ArrayBuffer]]> {
        if (this.size === 0 && this.offset >= 4) {
            this.size = new Uint32Array(this.buffer, 0, 1)[0];
            if (this.offset === this.size) {
                const buffer = this.buffer;
                this.buffer = null;
                this.size = 0;
                this.offset = 0;
                return [this.streamId, [false, buffer]];
            }
            if (this.offset > this.size) {
                const newBuffer = await this.nextBuffer();

                console.log(`@1 newBuffer.byteLength=${newBuffer.byteLength}, this.offset=${this.offset}, this.size=${this.size}`); // @DEV

                new Uint8Array(newBuffer, 0, this.offset - this.size).set(
                    new Uint8Array(this.buffer, this.size, this.offset - this.size),
                    0,
                );
                const buffer = this.buffer;
                this.buffer = newBuffer;
                this.offset -= this.size;
                this.size = 0;
                return [this.streamId, [false, buffer]];
            }
        }
        const { value: view, done } = await this.reader.read(
            new Uint8Array(
                this.buffer,
                this.offset,
                (this.size === 0 ? this.buffer.byteLength : this.size) -
                    this.offset,
            ),
        );
        if (done) {
            return [this.streamId, [true, null]];
        }
        this.buffer = view.buffer;
        this.offset += view.length;
        if (this.size === 0) {
            if (this.offset < 4) {
                return [this.streamId, [false, null]];
            }
            this.size = new Uint32Array(this.buffer, 0, 1)[0];
        }
        if (this.offset < this.size) {
            return [this.streamId, [false, null]];
        }
        if (this.offset === this.size) {
            const buffer = this.buffer;
            this.buffer = null;
            this.size = 0;
            this.offset = 0;
            return [this.streamId, [false, buffer]];
        }
        const newBuffer = await this.nextBuffer();


        console.log(`@2 this.buffer.byteLength=${this.buffer.byteLength} newBuffer.byteLength=${newBuffer.byteLength}, this.offset=${this.offset}, this.size=${this.size}`); // @DEV

        new Uint8Array(newBuffer, 0, this.offset - this.size).set(
            new Uint8Array(this.buffer, this.size, this.offset - this.size),
            0,
        );
        const buffer = this.buffer;
        this.buffer = newBuffer;
        this.offset -= this.size;
        this.size = 0;
        return [this.streamId, [false, buffer]];
    }

    async next(): Promise<[number, [boolean, ArrayBuffer]]> {
        if (this.buffer == null) {
            this.buffer = await this.nextBuffer();
        }
        return this.nextWithBuffer();
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

class BidirectionalStream extends UnidirectionalStream {
    writer: WritableStreamDefaultWriter;

    constructor(
        streamId: number,
        sourceId: number,
        bufferCount: number,
        bufferSize: number,
        reader: ReadableStreamBYOBReader,
        sender: WritableStream,
    ) {
        super(streamId, sourceId, bufferCount, bufferSize, reader);
        this.writer = sender.getWriter();
    }
}

const idToStream: Map<number, UnidirectionalStream | BidirectionalStream> =
    new Map();
let messageStreamId: number = null;
const streamIdToPromise: Map<number, Promise<[number, any]>> = new Map();

let transport: WebTransport = null;
let nextStreamId: number = 0;

async function connect(
    protocol: string,
    hostname: string,
    port: string,
    path: string,
) {
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
        const unidirectionalStreams = transport.incomingUnidirectionalStreams;
        const unidirectionalStreamsReader = unidirectionalStreams.getReader();
        const unidirectionalGeneratorStreamId = nextStreamId;
        ++nextStreamId;
        const bidirectionalStreams = transport.incomingBidirectionalStreams;
        const bidirectionalStreamsReader = bidirectionalStreams.getReader();
        const bidirectionalGeneratorStreamId = nextStreamId;
        ++nextStreamId;
        streamIdToPromise.set(
            unidirectionalGeneratorStreamId,
            unidirectionalStreamsReader
                .read()
                .then(stream => [unidirectionalGeneratorStreamId, stream]),
        );
        streamIdToPromise.set(
            bidirectionalGeneratorStreamId,
            bidirectionalStreamsReader
                .read()
                .then(stream => [bidirectionalGeneratorStreamId, stream]),
        );
        let descriptionBuffer = new ArrayBuffer(12);
        while (true) {
            const [streamId, result] = await Promise.race(
                streamIdToPromise.values(),
            );
            if (
                streamId === unidirectionalGeneratorStreamId ||
                streamId === bidirectionalGeneratorStreamId
            ) {
                if (result.done) {
                    throw new Error(`stream ${streamId} is done`);
                }

                console.log(`streamId=${streamId}`, result.value); // @DEV

                const reader: ReadableStreamBYOBReader = (
                    streamId === unidirectionalGeneratorStreamId
                        ? result.value
                        : result.value.readable
                ).getReader({ mode: "byob" });

                console.log(`streamId=${streamId} getReader worked`); // @DEV

                let descriptionOffset = 0;
                while (descriptionOffset < descriptionBuffer.byteLength) {
                    const { value: view, done } = await reader.read(
                        new Uint8Array(
                            descriptionBuffer,
                            descriptionOffset,
                            descriptionBuffer.byteLength - descriptionOffset,
                        ),
                    );
                    if (done) {
                        throw new Error(
                            `new stream from generator ${streamId} closed before sending an id`,
                        );
                    }
                    descriptionBuffer = view.buffer;
                    descriptionOffset += view.length;
                }
                const [sourceId, recommendedBufferCount, maximumLength] =
                    new Uint32Array(descriptionBuffer);
                console.log(
                    `new stream sourceId=${sourceId} recommendedBufferCount=${recommendedBufferCount} maximumLength=${maximumLength}`,
                ); // @DEV
                for (const stream of idToStream.values()) {
                    if (stream.sourceId === sourceId) {
                        throw new Error(`duplicated stream ${sourceId}`);
                    }
                }
                const stream =
                    sourceId === unidirectionalGeneratorStreamId
                        ? new UnidirectionalStream(
                              nextStreamId,
                              sourceId,
                              recommendedBufferCount,
                              maximumLength,
                              reader,
                          )
                        : new BidirectionalStream(
                              nextStreamId,
                              sourceId,
                              recommendedBufferCount,
                              maximumLength,
                              reader,
                              result.value.writable,
                          );
                ++nextStreamId;
                idToStream.set(stream.streamId, stream);
                if (sourceId === constants.MESSAGES_SOURCE_ID) {
                    messageStreamId = stream.streamId;
                }
                streamIdToPromise.set(stream.streamId, stream.next());
                streamIdToPromise.set(
                    streamId,
                    (sourceId === unidirectionalGeneratorStreamId
                        ? unidirectionalStreamsReader
                        : bidirectionalStreamsReader
                    )
                        .read()
                        .then(stream => [streamId, stream]),
                );
            } else {
                const stream = idToStream.get(streamId);
                if (stream == null) {
                    console.error(
                        `a promise returned the unknown stream id ${streamId}`,
                    );
                }
                const [done, buffer]: [boolean, ArrayBuffer] = result;
                if (done) {
                    streamIdToPromise.delete(streamId);
                    idToStream.delete(streamId);
                    // @DEV communicate with the main thread to delete associated workers
                } else {
                    if (buffer == null) {
                        streamIdToPromise.set(streamId, stream.next());
                    } else {
                        console.log(
                            `stream.sourceId=${stream.sourceId}, stream.streamId=${stream.streamId}`,
                        ); // @DEV
                        postMessage(
                            {
                                type:
                                    stream.sourceId ===
                                    constants.MESSAGES_SOURCE_ID
                                        ? constants.TRANSPORT_TO_MAIN_BUFFER
                                        : constants.TRANSPORT_TO_DECODE_BUFFER,
                                streamId: stream.streamId,
                                sourceId: stream.sourceId,
                                buffer,
                            },
                            { transfer: [buffer] },
                        );
                        streamIdToPromise.set(streamId, stream.next());
                    }
                }
            }
        }
    } catch (error) {
        console.error(error);
        streamIdToPromise.clear();
        idToStream.clear();
        messageStreamId = null;
        if (transport != null) {
            transport.close();
            transport = null;
        }
        postMessage({
            type: constants.TRANSPORT_TO_MAIN_CONNECTION_STATUS,
            status: "disconnected",
        });
        setTimeout(connect, 1000, protocol, hostname, port, path);
    }
}

let started = false;

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
            if (messageStreamId != null) {
                const stream = idToStream.get(messageStreamId);
                if (stream != null) {
                    const size = new Uint32Array(data.buffer, 0, 1)[0];
                    (stream as BidirectionalStream).writer.write(
                        new Uint8Array(data.buffer, 0, size),
                    );
                }
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
