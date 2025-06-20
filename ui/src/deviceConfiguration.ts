export interface Configuration {
    type: "inivation_davis346" | "prophesee_evk3_hd" | "prophesee_evk4";
    configuration: { [key: string]: any };
}

export interface IntegerParameter {
    type: "integer";
    name: string;
    description: string;
    value: number;
    minimum: number;
    maximum: number;
    default: number;
    update: (newValue: number) => void;
}

export interface BooleanParameter {
    type: "boolean";
    name: string;
    description: string;
    value: boolean;
    default: boolean;
    update: (newValue: boolean) => void;
}

function u8(
    parent: { [key: string]: any },
    name: string,
    description: string,
    defaultValue: number,
): IntegerParameter {
    return {
        type: "integer",
        name,
        description,
        value: parent[name],
        minimum: 0,
        maximum: 255,
        default: defaultValue,
        update: (newValue: number) => {
            parent[name] = newValue;
        },
    };
}

export function groupsAndParameters(
    configuration: Configuration,
): [string, (IntegerParameter | BooleanParameter)[]][] {
    switch (configuration.type) {
        case "inivation_davis346":
            return [];
        case "prophesee_evk3_hd":
            return [];
        case "prophesee_evk4":
            return [
                [
                    "Biases",
                    [
                        u8(
                            configuration.configuration.biases,
                            "pr",
                            "<strong>Photoreceptor.</strong> <em>Increasing</em> the value <em>increases</em> the speed with which the first stage responds to changes at the cost of a higher power consumption. In low-light conditions, the response speed is dictated by the scene illuminance.",
                            0x7c,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "fo",
                            "<strong>Low pass filter.</strong> <em>Increasing</em> the value <em>increases</em> the bandwidth, allowing for the detection of quicker fluctuations with lower latency but at the cost of letting more flicker noise through.",
                            0x53,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "hpf",
                            "<strong>High pass filter.</strong> <em>Increasing</em> the value <em>decreases</em> the bandwidth, which reduces background noise but discards slow illumination changes.",
                            0x00,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "diff_on",
                            "Contrast threshold for ON events. <em>Increasing</em> the value <em>increases</em> the threshold and thus reduces sensitivity.",
                            0x66,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "diff",
                            "<strong>Second stage amplifier.</strong> <em>Increasing</em> the value <em>increases</em> the speed with which the second stage responds to changes.",
                            0x4d,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "diff_off",
                            "<strong>Contrast threshold for OFF events.</strong> <em>Increasing</em> the value <em>increases</em> the threshold and thus reduces sensitivity.",
                            0x49,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "inv",
                            "<strong>Inverter.</strong> Not documented.",
                            0x5b,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "refr",
                            "<strong>Refractory period.</strong> <em>Increasing</em> the value <em>decreases</em> the refractory period (pixels can spike again more quickly).",
                            0x14,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "reqpuy",
                            "<strong>Y request pull-up</strong>. Impacts the arbiter's behavior.",
                            0x8c,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "reqpux",
                            "<strong>X request pull-up</strong>. Impacts the arbiter's behavior.",
                            0x7c,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "sendreqpdy",
                            "<strong>Send request pull-up</strong>. Impacts the arbiter's behavior.",
                            0x94,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "unknown_1",
                            "<strong>Unknown 1</strong>. Not documented.",
                            0x74,
                        ),
                        u8(
                            configuration.configuration.biases,
                            "unknown_2",
                            "<strong>Unknown 2</strong>. Not documented.",
                            0x51,
                        ),
                    ],
                ],
                [
                    "Region of interest",
                    [
                        {
                            type: "integer",
                            name: "x",
                            description:
                                "Horizontal starting position (from the left) of the region of interest (pixels outside of the ROI are disabled).",
                            value: 0,
                            minimum: 0,
                            maximum: 1279,
                            default: 0,
                            update: (newValue: number) => {},
                        },
                        {
                            type: "integer",
                            name: "y",
                            description:
                                "Vertical starting position (from the top) of the region of interest (pixels outside of the ROI are disabled).",
                            value: 0,
                            minimum: 0,
                            maximum: 719,
                            default: 0,
                            update: (newValue: number) => {},
                        },
                        {
                            type: "integer",
                            name: "width",
                            description:
                                "Horizontal size of the region of interest (pixels outside of the ROI are disabled).",
                            value: 1280,
                            minimum: 1,
                            maximum: 1280,
                            default: 1280,
                            update: (newValue: number) => {},
                        },
                        {
                            type: "integer",
                            name: "height",
                            description:
                                "Vertical size of the region of interest (pixels outside of the ROI are disabled).",
                            value: 720,
                            minimum: 1,
                            maximum: 720,
                            default: 720,
                            update: (newValue: number) => {},
                        },
                    ],
                ],
            ];
        default:
            throw new Error(
                `unsupported configuration type ${configuration.type}`,
            );
    }
}
