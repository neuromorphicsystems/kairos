import * as constants from "./constants";
import * as utilities from "./utilities";

// @ts-ignore
import extension_binary from "../extension/build/extension_bg.wasm";
import init, * as extension from "../extension/build/extension";

(async () => {
    const module = await init({ module_or_path: extension_binary.buffer });
    const renderer = new extension.Renderer(1280, 720, 1 << 22);
    const ts_and_ons_pointer_byte_length =
        renderer.ts_and_ons_pointer_byte_length();
    const buffers: ArrayBuffer[] = new Array(
        constants.DECODE_OUTPUT_BUFFERS_COUNT,
    )
        .fill(null)
        .map(_ => new ArrayBuffer(ts_and_ons_pointer_byte_length));
    postMessage({
        type: constants.DECODE_TO_MAIN_READY,
    });
    let stream = null;
    self.addEventListener("message", ({ data }) => {
        switch (data.type) {
            case constants.MAIN_TO_DECODE_SETUP: {
                stream = data.stream;
                break;
            }
            case constants.TRANSPORT_TO_DECODE_BUFFER: {
                if (stream == null) {
                    console.error(`decoder received a buffer before setup`);
                    self.postMessage(
                        {
                            type: constants.DECODE_TO_TRANSPORT_BUFFER,
                            streamId: data.streamId,
                            sourceId: data.sourceId,
                            buffer: data.buffer,
                        },
                        { transfer: [data.buffer] },
                    );
                } else {
                    const size = new Uint32Array(data.buffer, 0, 1)[0];
                    new Uint8Array(
                        module.memory.buffer,
                        renderer.evt3_buffer_pointer(),
                        size,
                    ).set(new Uint8Array(data.buffer, 0, size));
                    self.postMessage(
                        {
                            type: constants.DECODE_TO_TRANSPORT_BUFFER,
                            streamId: data.streamId,
                            sourceId: data.sourceId,
                            buffer: data.buffer,
                        },
                        { transfer: [data.buffer] },
                    );
                    extension.render(renderer, size);
                    const glCurrentT = renderer.gl_current_t();
                    const displayT = renderer.display_t();
                    if (buffers.length === 0) {
                        console.error("decoder ran out of output buffers");
                    } else {
                        const buffer = buffers.shift();
                        new Uint8Array(
                            buffer,
                            0,
                            ts_and_ons_pointer_byte_length,
                        ).set(
                            new Uint8Array(
                                module.memory.buffer,
                                renderer.ts_and_ons_pointer(),
                                ts_and_ons_pointer_byte_length,
                            ),
                        );
                        self.postMessage(
                            {
                                type: constants.DECODE_TO_PAINT_BUFFER,
                                buffer,
                                glCurrentT,
                                displayT,
                            },
                            { transfer: [buffer] },
                        );
                    }
                    break;
                }
            }
            case constants.PAINT_TO_DECODE_BUFFER: {
                buffers.push(data.buffer);
                break;
            }
            default: {
                console.error(
                    `unexpected message in decoder ${JSON.stringify(data)}`,
                );
            }
        }
    });
})();
