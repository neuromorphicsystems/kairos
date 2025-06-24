export const MAIN_TO_TRANSPORT_SETUP = 1;
export const MAIN_TO_TRANSPORT_BUFFER = 2;
export const MAIN_TO_TRANSPORT_MESSAGE = 3;
export const TRANSPORT_TO_MAIN_CONNECTION_STATUS = 4;
export const TRANSPORT_TO_MAIN_MESSAGE_BUFFER = 5;
export const TRANSPORT_TO_MAIN_RECORD_STATE_BUFFER = 6;
export const TRANSPORT_TO_DECODE_BUFFER = 7;
export const MAIN_TO_DECODE_SETUP = 8;
export const DECODE_TO_MAIN_READY = 9;
export const DECODE_TO_PAINT_BUFFER = 10;
export const DECODE_TO_TRANSPORT_BUFFER = 11;
export const PAINT_TO_DECODE_BUFFER = 12;

// export const RENDER_SETUP = 2;
// export const RENDER_TO_TRANSPORT = 4;
// export const RENDER_TO_DRAW = 5;
// export const DRAW_TO_RENDER = 6;

export const DECODE_OUTPUT_BUFFERS_COUNT: number = 16;
export const PING_INTERVAL: number = 1000.0;
export const MAXIMUM_SCALE: number = 50.0;
export const CLICK_MAXIMUM_DISTANCE: number = 3.0;
export const MAXIMUM_F32_VALUE: number = 3600000000;
export const MESSAGE_SOURCE_ID: number = 0xffffff00;
export const RECORD_STATE_SOURCE_ID: number = 0xffffff01;
export const CHART_AUTO_ORIENTATION_RATIO: number = 2.7;
export const AUTOTRIGGER_MAXIMUM_WINDOW_SIZE: number = 600;

export const CHART_RANGES = [0.0, 5.0, 10.0, 30.0, 60.0];

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

export type ContainerId = 0 | 1 | 2 | 3 | 4;

export type Layout =
    | "full"
    | "h"
    | "hv1"
    | "hv2"
    | "v"
    | "vh1"
    | "vh2"
    | "hv1v2"
    | "vh1h2";
