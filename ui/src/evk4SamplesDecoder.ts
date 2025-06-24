import * as constants from "./constants";
import SamplePainter from "./samplePainter";

class Evk4SamplesDecoder {
    painter: SamplePainter;
    handleBuffer: (
        streamId: number,
        sourceId: number,
        buffer: ArrayBuffer,
    ) => void;

    constructor(
        painter: SamplePainter,
        handleBuffer: (
            streamId: number,
            sourceId: number,
            buffer: ArrayBuffer,
        ) => void,
    ) {
        this.painter = painter;
        this.handleBuffer = handleBuffer;
    }

    postMessage(
        data: {
            type: number;
            streamId: number;
            sourceId: number;
            buffer: ArrayBuffer;
        },
        transferables: {
            transfer: ArrayBuffer[];
        },
    ) {
        const size = new Uint32Array(data.buffer, 0, 1)[0];
        if (size === 60) {
            const view = new DataView(data.buffer, 4, 60);
            const systemTime = view.getBigUint64(0, true);
            const systemTimestamp = view.getBigUint64(8, true);
            const onEventRate = view.getFloat32(16, true);
            const offEventRate = view.getFloat32(20, true);
            const risingTriggerCount = view.getUint32(24, true);
            const fallingTriggerCount = view.getUint32(28, true);
            const illuminance = view.getFloat32(32, true);
            const temperature = view.getFloat32(36, true);
            const autotriggerShortValue = view.getFloat32(40, true);
            const autotriggerLongValue = view.getFloat32(44, true);
            const autotriggerRatio = view.getFloat32(48, true);
            const autotriggerThreshold = view.getFloat32(52, true);
            const date = new Date(Number(systemTime / 1000n));
            const secondsDeciseconds = `${date.getUTCSeconds().toString().padStart(2, "0")}.${Math.floor(date.getUTCMilliseconds() / 100).toFixed(0)}`;
            this.painter.push(
                systemTime,
                secondsDeciseconds,
                `${date.getUTCFullYear().toString().padStart(4, "0")}-${(
                    date.getUTCMonth() + 1
                )
                    .toString()
                    .padStart(2, "0")}-${date
                    .getUTCDate()
                    .toString()
                    .padStart(2, "0")} ${date
                    .getUTCHours()
                    .toString()
                    .padStart(2, "0")}:${date
                    .getUTCMinutes()
                    .toString()
                    .padStart(2, "0")}:${secondsDeciseconds} UTC`,
                [
                    [onEventRate + offEventRate, onEventRate, offEventRate],
                    [illuminance],
                    [temperature],
                    [risingTriggerCount, fallingTriggerCount],
                    [
                        autotriggerShortValue,
                        autotriggerLongValue,
                        autotriggerRatio,
                        autotriggerThreshold,
                    ],
                ],
            );
        } else {
            console.error(
                `EVK4 sample has an unexpected size (${size} B, expected 44 B)`,
            );
        }
        this.handleBuffer(data.streamId, data.sourceId, data.buffer);
    }
}

export default Evk4SamplesDecoder;
