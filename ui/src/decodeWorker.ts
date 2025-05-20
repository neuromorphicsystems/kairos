import * as constants from "./constants";

// @ts-ignore
import extension_binary from "../extension/build/extension_bg.wasm";
import init, * as extension from "../extension/build/extension";

console.log("decode created"); // @DEV

(async () => {
    console.log("decode async created"); // @DEV

    const module = await init({ module_or_path: extension_binary.buffer });
    const renderer = new extension.Renderer(1280, 720, 1 << 24);
    const ts_and_ons_pointer_byte_length =
        renderer.ts_and_ons_pointer_byte_length();
    const buffers: ArrayBuffer[] = new Array(16)
        .fill(null)
        .map(_ => new ArrayBuffer(ts_and_ons_pointer_byte_length));
    postMessage({
        type: constants.DECODE_TO_MAIN_READY,
    });
    let stream = null;

    let start = null; // @DEV
    let index = 0; // @DEV
    let maximumDelta = 0; // @DEV
    let maximumRenderTime = 0; // @DEV
    self.addEventListener("message", ({ data }) => {
        switch (data.type) {
            case constants.MAIN_TO_DECODE_SETUP: {
                stream = data.stream;
                break;
            }
            case constants.TRANSPORT_TO_DECODE_BUFFER: {
                if (stream == null) {
                    console.error(`decode received a buffer before setup`);
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
                    const a = performance.now(); // @DEV
                    const size = new Uint32Array(data.buffer)[0];
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
                    const currentT = renderer.current_t();
                    const glCurrentT = renderer.gl_current_t();
                    if (buffers.length === 0) {
                        console.error("decode ran out of output buffers");
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
                                currentT,
                                glCurrentT,
                            },
                            { transfer: [buffer] },
                        );
                    }

                    // @DEV {
                    const b = performance.now();
                    const renderTime = b - a;
                    maximumRenderTime = Math.max(renderTime, maximumRenderTime);
                    const now = performance.now();
                    if (start == null) {
                        start = now;
                    }
                    ++index;
                    const delta =
                        now - start + 1000.0 / 60.0 - (index * 1000.0) / 60.0;
                    maximumDelta = Math.max(Math.abs(delta), maximumDelta);
                    console.log(
                        `${index}, expected = ${((index * 1000.0) / 60.0).toFixed(3)} ms, actual = ${(now - start + 1000.0 / 60.0).toFixed(3)}, delta = ${delta.toFixed(3)} ms, received ${size} B, render time = ${renderTime.toFixed(3)} ms, max render time = ${maximumRenderTime.toFixed(3)} ms, max delta = ${maximumDelta.toFixed(3)} ms, current_t = ${renderer.current_t()}`,
                    );
                    // }
                    break;
                }
            }
            case constants.PAINT_TO_DECODE_BUFFER: {
                buffers.push(data.buffer);
                break;
            }
            default: {
                console.error(
                    `unexpected message in decode ${JSON.stringify(data)}`,
                );
            }
        }
    });
})();
