import type { SampleDisplayProperties } from "./appState.svelte";

import * as constants from "./constants";
import * as chartjs from "chart.js/auto";

chartjs.Chart.defaults.font.family = '"Roboto", sans-serif';

interface ChartProperties {
    name: string;
    yLabel: string;
    logarithmic: boolean;
    curvesNamesAndColors: [string, string][];
}

class Collection {
    range: number;
    timestamps: number[];
    labels: string[];
    extendedLabels: string[];
    chartToCurvesToValues: number[][][];

    constructor(rangeIndex: number, chartsProperties: ChartProperties[]) {
        this.range = Math.round(constants.CHART_RANGES[rangeIndex] * 10);
        this.timestamps = [];
        this.labels = [];
        this.extendedLabels = [];
        this.chartToCurvesToValues = chartsProperties.map(properties =>
            properties.curvesNamesAndColors.map(_ => []),
        );
    }

    push(
        timestamp: bigint,
        label: string,
        extendedLabel: string,
        chartToCurvesToValue: number[][],
    ) {
        const timestampDeciseconds = Number(timestamp / 100000n);
        while (
            this.timestamps.length > 0 &&
            timestampDeciseconds - this.timestamps[0] >= this.range
        ) {
            this.timestamps.shift();
            this.labels.shift();
            this.extendedLabels.shift();
            for (const curveToValues of this.chartToCurvesToValues) {
                for (const values of curveToValues) {
                    values.shift();
                }
            }
        }
        this.timestamps.push(timestampDeciseconds);
        this.labels.push(label);
        this.extendedLabels.push(extendedLabel);
        for (
            let chartIndex = 0;
            chartIndex < this.chartToCurvesToValues.length;
            ++chartIndex
        ) {
            for (
                let curveIndex = 0;
                curveIndex < this.chartToCurvesToValues[chartIndex].length;
                ++curveIndex
            ) {
                this.chartToCurvesToValues[chartIndex][curveIndex].push(
                    chartToCurvesToValue[chartIndex][curveIndex],
                );
            }
        }
    }
}

class Chart {
    name: string;
    inner: chartjs.Chart<"line", number[], string>;

    constructor(
        canvas: HTMLCanvasElement,
        properties: ChartProperties,
        collection: Collection,
        chartIndex: number,
    ) {
        this.name = properties.name;
        this.inner = new chartjs.Chart(canvas.getContext("2d"), {
            options: {
                normalized: true,
                animation: false,
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    x: {
                        display: true,
                        ticks: {
                            maxRotation: 0,
                            autoSkipPadding: 20,
                            font: {
                                family: '"Roboto Mono", monospace',
                            },
                            sampleSize: 1,
                            color: "#aaaaaa",
                        },
                        grid: {
                            color: "#333333",
                        },
                        border: {
                            color: "#aaaaaa",
                        },
                    },
                    y: {
                        display: true,
                        type: properties.logarithmic ? "logarithmic" : "linear",
                        ticks: {
                            maxRotation: 0,
                            autoSkipPadding: 20,
                            font: {
                                family: '"Roboto Mono", monospace',
                            },
                            color: "#aaaaaa",
                        },
                        grid: {
                            color: "#333333",
                        },
                        border: {
                            color: "#aaaaaa",
                        },
                        title: {
                            display: true,
                            text: properties.yLabel,
                            color: "#aaaaaa",
                            padding: 10,
                            font: {
                                size: 14,
                            },
                        },
                    },
                },
                interaction: {
                    mode: "nearest",
                    axis: "x",
                    intersect: false,
                },
                elements: {
                    point: {
                        radius: 0,
                    },
                },
                plugins: {
                    legend: {
                        display: false,
                    },
                    tooltip: {
                        callbacks: {
                            title: context =>
                                collection.extendedLabels[context[0].dataIndex],
                        },
                    },
                },
            },
            type: "line",
            data: {
                labels: collection.labels,
                datasets: properties.curvesNamesAndColors.map(
                    (nameAndColor, index) => ({
                        spanGaps: false,
                        label: nameAndColor[0],
                        backgroundColor: nameAndColor[1],
                        borderColor: nameAndColor[1],
                        borderJoinStyle: "round",
                        data: collection.chartToCurvesToValues[chartIndex][
                            index
                        ],
                    }),
                ),
            },
        });
    }
}

class Context {
    element: HTMLElement;
    properties: SampleDisplayProperties;
    charts: Chart[];
    currentHash: number;

    constructor(element: HTMLElement, properties: SampleDisplayProperties) {
        this.element = element;
        this.properties = properties;
        this.charts = [];
        this.currentHash = 0;
    }

    updateHTML(chartsProperties: ChartProperties[], collections: Collection[]) {
        if (this.properties.hash === this.currentHash) {
            return;
        }
        if (
            this.properties.namesAndRangesIndices.length !==
                chartsProperties.length ||
            chartsProperties.some(
                (chartProperties, index) =>
                    chartProperties.name !==
                    this.properties.namesAndRangesIndices[index][0],
            )
        ) {
            throw new Error(
                `Mismatch betweeb chart properties (${JSON.stringify(
                    chartsProperties,
                )}) and namesAndRangesIndices (${JSON.stringify(
                    this.properties.namesAndRangesIndices,
                )})`,
            );
        }
        for (const chart of this.charts) {
            chart.inner.destroy();
        }
        this.charts = [];
        this.element.replaceChildren();
        for (
            let chartIndex = 0;
            chartIndex < chartsProperties.length;
            ++chartIndex
        ) {
            if (this.properties.namesAndRangesIndices[chartIndex][1] > 0) {
                const container = document.createElement("div");
                container.style.overflow = "hidden";
                container.style.flexShrink = "1";
                container.style.flexGrow = "1";
                container.style.position = "relative";
                this.element.appendChild(container);
                const canvas = document.createElement("canvas");
                canvas.style.position = "absolute";
                canvas.style.top = "0";
                canvas.style.left = "0";
                container.appendChild(canvas);
                this.charts.push(
                    new Chart(
                        canvas,
                        chartsProperties[chartIndex],
                        collections[
                            this.properties.namesAndRangesIndices[chartIndex][1]
                        ],
                        chartIndex,
                    ),
                );
            }
        }
        this.currentHash = this.properties.hash;
    }
}

class SamplePainter {
    type: "SamplePainter";
    nextElementId: number;
    contextsAndIds: [Context, number][];
    chartsProperties: ChartProperties[];
    timestamps: bigint[];
    collections: Collection[];
    running: boolean;
    update: boolean;

    constructor(chartsProperties: ChartProperties[]) {
        this.type = "SamplePainter";
        this.nextElementId = 0;
        this.contextsAndIds = [];
        this.chartsProperties = chartsProperties;
        this.timestamps = [];

        this.collections = [null];
        for (
            let rangeIndex = 1;
            rangeIndex < constants.CHART_RANGES.length;
            ++rangeIndex
        ) {
            this.collections.push(new Collection(rangeIndex, chartsProperties));
        }
        this.running = true;
        this.update = true;
        requestAnimationFrame(timestamp => {
            this.tick(timestamp);
        });
    }

    push(
        timestamp: bigint,
        label: string,
        extendedLabel: string,
        chartToCurvesToValue: number[][],
    ) {
        if (this.chartsProperties.length !== chartToCurvesToValue.length) {
            throw new Error(
                `expected ${this.chartsProperties.length} values in chartToCurvesToValue, got ${chartToCurvesToValue.length}`,
            );
        }
        for (
            let chartIndex = 0;
            chartIndex < this.chartsProperties.length;
            ++chartIndex
        ) {
            const expected =
                this.chartsProperties[chartIndex].curvesNamesAndColors.length;
            const got = chartToCurvesToValue[chartIndex].length;
            if (expected !== got) {
                throw new Error(
                    `expected ${expected} values for chart ${chartIndex}, got ${got}`,
                );
            }
        }
        for (const collection of this.collections) {
            if (collection != null) {
                collection.push(
                    timestamp,
                    label,
                    extendedLabel,
                    chartToCurvesToValue,
                );
            }
        }
        this.update = true;
    }

    attach(element: HTMLElement, properties: SampleDisplayProperties): number {
        for (const [existingContext, _] of this.contextsAndIds) {
            if (existingContext.element === element) {
                throw new Error(`${element} is already attached`);
            }
        }
        const elementId = this.nextElementId;
        ++this.nextElementId;
        const context = new Context(element, properties);
        context.updateHTML(this.chartsProperties, this.collections);
        this.contextsAndIds.push([context, elementId]);
        return elementId;
    }

    detach(elementId: number) {
        for (let index = 0; index < this.contextsAndIds.length; ++index) {
            if (elementId === this.contextsAndIds[index][1]) {
                for (const chart of this.contextsAndIds[index][0].charts) {
                    chart.inner.destroy();
                }
                this.contextsAndIds[index][0].element.replaceChildren();
                this.contextsAndIds.splice(index, 1);
                return;
            }
        }
        throw new Error(`${elementId} is not attached`);
    }

    tick(timestamp: number) {
        if (this.running) {
            for (const [context, _] of this.contextsAndIds) {
                context.updateHTML(this.chartsProperties, this.collections);
            }
            if (this.update) {
                for (const contextAndId of this.contextsAndIds) {
                    for (const chart of contextAndId[0].charts) {
                        chart.inner.update();
                    }
                }
                this.update = false;
            }
            requestAnimationFrame(timestamp => {
                this.tick(timestamp);
            });
        }
    }
}

export default SamplePainter;
