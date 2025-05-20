export const MAIN_TO_TRANSPORT_SETUP = 1;
export const MAIN_TO_TRANSPORT_BUFFER = 2;
export const MAIN_TO_TRANSPORT_MESSAGE = 3;
export const TRANSPORT_TO_MAIN_CONNECTION_STATUS = 4;
export const TRANSPORT_TO_MAIN_BUFFER = 5;
export const TRANSPORT_TO_DECODE_BUFFER = 6;
export const MAIN_TO_DECODE_SETUP = 7;
export const DECODE_TO_MAIN_READY = 8;
export const DECODE_TO_PAINT_BUFFER = 9;
export const DECODE_TO_TRANSPORT_BUFFER = 10;
export const PAINT_TO_DECODE_BUFFER = 11;

// export const RENDER_SETUP = 2;
// export const RENDER_TO_TRANSPORT = 4;
// export const RENDER_TO_DRAW = 5;
// export const DRAW_TO_RENDER = 6;

export const MAXIMUM_DELTA = 3600000000;
export const MESSAGES_SOURCE_ID: number = 0xffffff;

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
