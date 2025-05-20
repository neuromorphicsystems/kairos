import * as constants from "./constants";

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
    if (t_and_on < ${constants.MAXIMUM_DELTA * 2}.0f) {
        if (style == 0) {
            lambda = exp(-float(current_t - t) / tau);
        } else if (style == 1) {
            lambda = (current_t - t) < tau ? 1.0f - (current_t - t) / tau : 0.0f;
        } else {
            lambda = (current_t - t) < tau ? 1.0f : 0.0f;
        }
    }
    color = texture(color_sampler, vec2(colormap_split * (1.0f - lambda) + (on ? lambda : 0.0f), 0.5f));
}
`;

interface Context {
    canvas: HTMLCanvasElement;
    configuration: {
        onColormap: [number, number, number, number][];
        offColormap: [number, number, number, number][];
        colormapChanged: boolean;
        colormapSplit: number;
        style: number;
        tau: number;
    };
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

function hexToRgba(hex: string): [number, number, number, number] {
    if (hex.startsWith("#")) {
        if (hex.length === 7) {
            return [
                parseInt(hex.slice(1, 3), 16),
                parseInt(hex.slice(3, 5), 16),
                parseInt(hex.slice(5, 7), 16),
                255,
            ];
        }
        if (hex.length === 9) {
            return [
                parseInt(hex.slice(1, 3), 16),
                parseInt(hex.slice(3, 5), 16),
                parseInt(hex.slice(5, 7), 16),
                parseInt(hex.slice(7, 9), 16),
            ];
        }
    }
    throw new Error(`parsing ${hex} as a color failed`);
}

function newContext(canvas: HTMLCanvasElement): Context {
    return {
        canvas,
        gl: null,
        program: null,
        configuration: {
            onColormap: [hexToRgba("#191919"), hexToRgba("#FBBC05")],
            offColormap: [hexToRgba("#191919"), hexToRgba("#4285F4")],
            colormapChanged: true,
            colormapSplit: 0,
            style: 0,
            tau: 500000.0,
        },
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
    currentT: number,
    glCurrentT: number,
) {
    if (context.gl == null) {
        context.gl = context.canvas.getContext("webgl2");
        context.gl.getExtension("OES_texture_float_linear");
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
            context.gl.LINEAR,
        );
        context.gl.texParameteri(
            context.gl.TEXTURE_2D,
            context.gl.TEXTURE_MAG_FILTER,
            context.gl.NEAREST,
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
    }
    context.gl.useProgram(context.program);
    context.gl.uniform1f(context.location.currentT, glCurrentT);
    context.gl.uniform1i(context.location.style, context.configuration.style);
    context.gl.uniform1f(context.location.tau, context.configuration.tau);
    context.gl.disable(context.gl.DEPTH_TEST);
    context.gl.enable(context.gl.BLEND);
    context.gl.blendFunc(context.gl.SRC_ALPHA, context.gl.ONE_MINUS_SRC_ALPHA);
    context.gl.clearColor(0.2, 0.3, 0.4, 1.0);
    context.gl.clear(context.gl.COLOR_BUFFER_BIT | context.gl.DEPTH_BUFFER_BIT);
    context.gl.activeTexture(context.gl.TEXTURE0);
    context.gl.bindTexture(context.gl.TEXTURE_2D, context.tsAndOnsTexture);
    context.gl.uniform1i(context.location.tAndOnSampler, 0);
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
    context.gl.activeTexture(context.gl.TEXTURE1);
    context.gl.bindTexture(context.gl.TEXTURE_2D, context.colormapTexture);
    context.gl.uniform1i(context.location.colorSampler, 1);
    if (context.configuration.colormapChanged) {
        const colormapLength =
            context.configuration.onColormap.length +
            context.configuration.offColormap.length;
        const colormapData = new Uint8Array(colormapLength * 4);
        let index = (context.configuration.offColormap.length - 1) * 4;
        for (const [r, g, b, a] of context.configuration.offColormap) {
            colormapData[index] = r;
            colormapData[index + 1] = g;
            colormapData[index + 2] = b;
            colormapData[index + 3] = a;
            index -= 4;
        }
        index = context.configuration.offColormap.length * 4;
        for (const [r, g, b, a] of context.configuration.onColormap) {
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
        context.configuration.colormapSplit =
            colormapLength === 0
                ? 0.0
                : context.configuration.offColormap.length / colormapLength;
        context.configuration.colormapChanged = false;
        console.log(context); // @DEV
    }
    context.gl.uniform1f(
        context.location.colormapSplit,
        context.configuration.colormapSplit,
    );
    context.gl.bindVertexArray(context.vertexArrayObject);
    context.gl.drawArrays(context.gl.TRIANGLE_STRIP, 0, 4);
}

export class Painter {
    decodeWorker: Worker;
    width: number;
    height: number;
    nextCanvasId: number;
    contextsAndIds: [Context, number][];
    buffersAndTimes: [ArrayBuffer, number, number][];
    previousTimestamp: number;

    constructor(decodeWorker: Worker, width: number, height: number) {
        this.decodeWorker = decodeWorker;
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
                const [buffer, currentT, glCurrentT] =
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
                            currentT,
                            glCurrentT,
                        );
                    }
                }
                this.decodeWorker.postMessage(
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

    handleBuffer(data: ArrayBuffer, currentT: number, glCurrentT: number) {
        this.buffersAndTimes.push([data, currentT, glCurrentT]);
        while (this.buffersAndTimes.length > 4) {
            const [buffer, _currentT, _glCurrentT] =
                this.buffersAndTimes.shift();
            this.decodeWorker.postMessage(
                {
                    type: constants.PAINT_TO_DECODE_BUFFER,
                    buffer,
                },
                { transfer: [buffer] },
            );
        }
    }

    attachCanvas(canvas: HTMLCanvasElement): number {
        for (const [existingContext, _] of this.contextsAndIds) {
            if (existingContext.canvas === canvas) {
                throw new Error(`${canvas} is already attached`);
            }
        }
        canvas.width = this.width;
        canvas.height = this.height;
        const canvasId = this.nextCanvasId;
        ++this.nextCanvasId;
        this.contextsAndIds.push([newContext(canvas), canvasId]);
        return canvasId;
    }

    detachCanvas(canvasId: number) {
        for (let index = 0; index < this.contextsAndIds.length; ++index) {
            if (canvasId === this.contextsAndIds[index][1]) {
                this.contextsAndIds.splice(index, 1);
                return;
            }
        }
        throw new Error(`${canvasId} is not attached`);
    }
}
