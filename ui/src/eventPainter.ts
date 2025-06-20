import type { EventDisplayProperties } from "./appState.svelte";

import * as constants from "./constants";
import * as utilities from "./utilities";
import { namedColormaps } from "./colormaps";

const vertexShaderSource: string = `#version 300 es
precision highp float;

in vec2 vertices;
out vec2 coordinates;

void main() {
    gl_Position = vec4(vertices.x * 2.0 - 1.0, 1.0 - vertices.y * 2.0, 0.0, 1.0);
    coordinates = vertices;
}
`;

const fragmentShaderSource: string = `#version 300 es
precision highp float;

in vec2 coordinates;
out vec4 color;
uniform sampler2D t_and_on_sampler;
uniform sampler2D color_sampler;
uniform float colormap_split;
uniform float current_t;
uniform int style;
uniform float tau;

void main() {
    float t_and_on = texture(t_and_on_sampler, coordinates).r;
    float t = abs(t_and_on);
    bool on = t_and_on >= 0.0f;
    float lambda = 0.0f;
    if (t_and_on < ${constants.MAXIMUM_F32_VALUE * 2}.0f) {
        if (style == 0) {
            lambda = exp(-float(current_t - t) / tau);
        } else if (style == 1) {
            lambda = (current_t - t) < (tau * 2.0f) ? 1.0f - (current_t - t) / (tau * 2.0f) : 0.0f;
        } else {
            lambda = (current_t - t) < tau ? 1.0f : 0.0f;
        }
    }
    color = texture(color_sampler, vec2(colormap_split * (1.0f - lambda) + (on ? lambda : 0.0f), 0.5f));
}
`;

interface Context {
    canvas: HTMLCanvasElement;
    timestampOverlay: HTMLElement;
    previousDisplayT: string;
    properties: EventDisplayProperties;
    cachedColormapIndex: number;
    colormapSplit: number;
    gl: WebGL2RenderingContext;
    program: WebGLProgram;
    location: {
        vertices: number;
        tAndOnSampler: WebGLUniformLocation;
        colorSampler: WebGLUniformLocation;
        colormapSplit: WebGLUniformLocation;
        currentT: WebGLUniformLocation;
        style: WebGLUniformLocation;
        tau: WebGLUniformLocation;
    };
    vertexArrayObject: WebGLVertexArrayObject;
    verticesBuffer: WebGLBuffer;
    tsAndOnsTexture: WebGLTexture;
    colormapTexture: WebGLTexture;
}

function newContext(
    canvas: HTMLCanvasElement,
    timestampOverlay: HTMLElement,
    properties: EventDisplayProperties,
): Context {
    return {
        canvas,
        timestampOverlay,
        previousDisplayT: "",
        properties,
        cachedColormapIndex: null,
        colormapSplit: null,
        gl: null,
        program: null,
        location: {
            vertices: null,
            tAndOnSampler: null,
            colorSampler: null,
            colormapSplit: null,
            currentT: null,
            style: null,
            tau: null,
        },
        vertexArrayObject: null,
        verticesBuffer: null,
        tsAndOnsTexture: null,
        colormapTexture: null,
    };
}

function paint(
    width: number,
    height: number,
    context: Context,
    data: Float32Array,
    glCurrentT: number,
    displayT: string,
) {
    if (context.gl == null) {
        context.gl = context.canvas.getContext("webgl2");
        if (context.gl == null) {
            throw new Error("creating a webgl2 context failed");
        }
    }
    if (context.program == null) {
        const vertexShader = context.gl.createShader(context.gl.VERTEX_SHADER);
        context.gl.shaderSource(vertexShader, vertexShaderSource);
        context.gl.compileShader(vertexShader);
        if (
            !context.gl.getShaderParameter(
                vertexShader,
                context.gl.COMPILE_STATUS,
            )
        ) {
            const message = `Vertex shader compilation error: ${context.gl.getShaderInfoLog(vertexShader)}`;
            context.gl.deleteShader(vertexShader);
            throw new Error(message);
        }
        const fragmentShader = context.gl.createShader(
            context.gl.FRAGMENT_SHADER,
        );
        context.gl.shaderSource(fragmentShader, fragmentShaderSource);
        context.gl.compileShader(fragmentShader);
        if (
            !context.gl.getShaderParameter(
                fragmentShader,
                context.gl.COMPILE_STATUS,
            )
        ) {
            const message = `Fragment shader compilation error: ${context.gl.getShaderInfoLog(fragmentShader)}`;
            context.gl.deleteShader(fragmentShader);
            throw new Error(message);
        }
        const program = context.gl.createProgram();
        context.gl.attachShader(program, vertexShader);
        context.gl.attachShader(program, fragmentShader);
        context.gl.linkProgram(program);
        if (!context.gl.getProgramParameter(program, context.gl.LINK_STATUS)) {
            throw new Error(
                `Program link error: ${context.gl.getProgramInfoLog(program)}`,
            );
        }
        context.location = {
            vertices: context.gl.getAttribLocation(program, "vertices"),
            tAndOnSampler: context.gl.getUniformLocation(
                program,
                "t_and_on_sampler",
            ),
            colorSampler: context.gl.getUniformLocation(
                program,
                "color_sampler",
            ),
            colormapSplit: context.gl.getUniformLocation(
                program,
                "colormap_split",
            ),
            currentT: context.gl.getUniformLocation(program, "current_t"),
            style: context.gl.getUniformLocation(program, "style"),
            tau: context.gl.getUniformLocation(program, "tau"),
        };
        context.vertexArrayObject = context.gl.createVertexArray();
        context.gl.bindVertexArray(context.vertexArrayObject);
        context.gl.enableVertexAttribArray(context.location.vertices);
        context.verticesBuffer = context.gl.createBuffer();
        context.gl.bindBuffer(context.gl.ARRAY_BUFFER, context.verticesBuffer);
        context.gl.bufferData(
            context.gl.ARRAY_BUFFER,
            new Float32Array([-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0]),
            context.gl.STATIC_DRAW,
        );
        context.gl.vertexAttribPointer(
            context.location.vertices,
            2,
            context.gl.FLOAT,
            false,
            0,
            0,
        );
        context.tsAndOnsTexture = context.gl.createTexture();
        context.gl.bindTexture(context.gl.TEXTURE_2D, context.tsAndOnsTexture);
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_WRAP_S,
            context.gl.CLAMP_TO_EDGE,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_WRAP_T,
            context.gl.CLAMP_TO_EDGE,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_MIN_FILTER,
            context.gl.NEAREST,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_MAG_FILTER,
            context.gl.NEAREST,
        );
        context.gl.texImage2D(
            context.gl.TEXTURE_2D,
            0,
            context.gl.R32F,
            width,
            height,
            0,
            context.gl.RED,
            context.gl.FLOAT,
            data,
            0,
        );
        context.colormapTexture = context.gl.createTexture();
        context.gl.bindTexture(context.gl.TEXTURE_2D, context.colormapTexture);
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_WRAP_S,
            context.gl.CLAMP_TO_EDGE,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_WRAP_T,
            context.gl.CLAMP_TO_EDGE,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_MIN_FILTER,
            context.gl.LINEAR,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_MAG_FILTER,
            context.gl.LINEAR,
        );
        context.program = program;

        context.gl.bindTexture(context.gl.TEXTURE_2D, context.tsAndOnsTexture); // WTF?
    } else {
        context.gl.bindTexture(context.gl.TEXTURE_2D, context.tsAndOnsTexture);
        context.gl.texSubImage2D(
            context.gl.TEXTURE_2D,
            0,
            0,
            0,
            width,
            height,
            context.gl.RED,
            context.gl.FLOAT,
            data,
            0,
        );
    }
    context.gl.useProgram(context.program);
    context.gl.uniform1f(context.location.currentT, glCurrentT);
    context.gl.uniform1i(context.location.style, context.properties.style);
    context.gl.uniform1f(context.location.tau, context.properties.tau);
    context.gl.disable(context.gl.DEPTH_TEST);
    context.gl.clearColor(0.0, 0.0, 0.0, 1.0);
    context.gl.clear(context.gl.COLOR_BUFFER_BIT | context.gl.DEPTH_BUFFER_BIT);
    context.gl.activeTexture(context.gl.TEXTURE0);
    context.gl.uniform1i(context.location.tAndOnSampler, 0);
    context.gl.activeTexture(context.gl.TEXTURE1);
    context.gl.bindTexture(context.gl.TEXTURE_2D, context.colormapTexture);
    context.gl.uniform1i(context.location.colorSampler, 1);
    if (context.cachedColormapIndex !== context.properties.colormapIndex) {
        const colormap = namedColormaps[context.properties.colormapIndex];
        const colormapLength = colormap.on.length + colormap.off.length;
        const colormapData = new Uint8Array(colormapLength * 4);
        let index = (colormap.off.length - 1) * 4;
        for (const [r, g, b, a] of colormap.off) {
            colormapData[index] = r;
            colormapData[index + 1] = g;
            colormapData[index + 2] = b;
            colormapData[index + 3] = a;
            index -= 4;
        }
        index = colormap.off.length * 4;
        for (const [r, g, b, a] of colormap.on) {
            colormapData[index] = r;
            colormapData[index + 1] = g;
            colormapData[index + 2] = b;
            colormapData[index + 3] = a;
            index += 4;
        }
        context.gl.texImage2D(
            context.gl.TEXTURE_2D,
            0,
            context.gl.RGBA8,
            colormapLength,
            1,
            0,
            context.gl.RGBA,
            context.gl.UNSIGNED_BYTE,
            colormapData,
            0,
        );
        context.colormapSplit =
            colormapLength === 0 ? 0.0 : colormap.off.length / colormapLength;
        context.cachedColormapIndex = context.properties.colormapIndex;
    }
    context.gl.uniform1f(context.location.colormapSplit, context.colormapSplit);
    context.gl.bindVertexArray(context.vertexArrayObject);
    context.gl.drawArrays(context.gl.TRIANGLE_STRIP, 0, 4);
    if (displayT !== context.previousDisplayT) {
        context.timestampOverlay.innerHTML = displayT;
        context.previousDisplayT = displayT;
    }
}

class EventPainter {
    type: "EventPainter";
    decoder: Worker;
    width: number;
    height: number;
    nextCanvasId: number;
    contextsAndIds: [Context, number][];
    buffersAndTimes: [ArrayBuffer, number, string][];
    previousTimestamp: number;

    constructor(decoder: Worker, width: number, height: number) {
        this.type = "EventPainter";
        this.decoder = decoder;
        this.width = width;
        this.height = height;
        this.nextCanvasId = 0;
        this.contextsAndIds = [];
        this.buffersAndTimes = [];
        this.previousTimestamp = null;
        requestAnimationFrame(timestamp => {
            this.tick(timestamp);
        });
    }

    tick(timestamp: number) {
        let frameCount = this.buffersAndTimes.length;
        if (this.previousTimestamp != null) {
            const elapsed = timestamp - this.previousTimestamp;
            frameCount = Math.min(
                this.buffersAndTimes.length,
                Math.round(elapsed * (60.0 / 1000.0)),
            );
        }
        if (frameCount > 0) {
            for (let index = 0; index < frameCount; ++index) {
                const [buffer, glCurrentT, displayT] =
                    this.buffersAndTimes.shift();
                if (index === frameCount - 1) {
                    const data = new Float32Array(
                        buffer,
                        0,
                        this.width * this.height,
                    );
                    for (const [context, _] of this.contextsAndIds) {
                        paint(
                            this.width,
                            this.height,
                            context,
                            data,
                            glCurrentT,
                            displayT,
                        );
                    }
                }
                this.decoder.postMessage(
                    {
                        buffer,
                        type: constants.PAINT_TO_DECODE_BUFFER,
                    },
                    { transfer: [buffer] },
                );
            }
            this.previousTimestamp = timestamp;
        }
        requestAnimationFrame(timestamp => {
            this.tick(timestamp);
        });
    }

    handleBuffer(data: ArrayBuffer, glCurrentT: number, displayT: string) {
        this.buffersAndTimes.push([data, glCurrentT, displayT]);
        while (this.buffersAndTimes.length > 4) {
            const [buffer, _glCurrentT, _displayT] =
                this.buffersAndTimes.shift();
            this.decoder.postMessage(
                {
                    type: constants.PAINT_TO_DECODE_BUFFER,
                    buffer,
                },
                { transfer: [buffer] },
            );
        }
    }

    attach(
        canvas: HTMLCanvasElement,
        timestampOverlay: HTMLElement,
        properties: EventDisplayProperties,
    ): number {
        for (const [existingContext, _] of this.contextsAndIds) {
            if (existingContext.canvas === canvas) {
                throw new Error(`${canvas} is already attached`);
            }
        }
        canvas.width = this.width;
        canvas.height = this.height;
        const canvasId = this.nextCanvasId;
        ++this.nextCanvasId;
        this.contextsAndIds.push([
            newContext(canvas, timestampOverlay, properties),
            canvasId,
        ]);

        console.log(
            `${utilities.utcString()} | attached ${canvasId}, props=${JSON.stringify(properties)}`,
        ); // @DEV

        return canvasId;
    }

    detach(canvasId: number) {
        console.log(`${utilities.utcString()} | detach ${canvasId}`); // @DEV

        for (let index = 0; index < this.contextsAndIds.length; ++index) {
            if (canvasId === this.contextsAndIds[index][1]) {
                this.contextsAndIds.splice(index, 1);
                return;
            }
        }
        throw new Error(`${canvasId} is not attached`);
    }
}

export default EventPainter;
