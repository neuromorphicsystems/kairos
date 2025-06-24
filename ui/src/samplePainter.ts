import type { SampleDisplayProperties } from "./appState.svelte";

import * as constants from "./constants";
import * as chartjs from "chart.js/auto";

chartjs.Chart.defaults.font.family = '"Roboto", sans-serif';

interface Curve {
    name: string;
    color: string;
    dash: [number, number] | null;
    axis: 0 | 1;
    order: number;
}

interface YAxis {
    label: string;
    units: string | null;
    logarithmic: boolean;
}

interface ChartProperties {
    name: string;
    yAxes: [YAxis, YAxis | null];
    curves: Curve[];
}

class Collection {
    range: number;
    timestamps: number[];
    labels: string[];
    extendedLabels: string[];
    chartToCurvesToData: { x: number; y: number }[][][];

    constructor(rangeIndex: number, chartsProperties: ChartProperties[]) {
        this.range = Math.round(constants.CHART_RANGES[rangeIndex] * 10);
        this.timestamps = [];
        this.labels = [];
        this.extendedLabels = [];
        this.chartToCurvesToData = chartsProperties.map(properties =>
            properties.curves.map(_ => []),
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
            timestampDeciseconds - this.timestamps[0] > this.range + 10
        ) {
            this.timestamps.shift();
            this.labels.shift();
            this.extendedLabels.shift();
            for (const curveToData of this.chartToCurvesToData) {
                for (const data of curveToData) {
                    data.shift();
                }
            }
        }
        this.timestamps.push(timestampDeciseconds);
        this.labels.push(label);
        this.extendedLabels.push(extendedLabel);
        for (
            let chartIndex = 0;
            chartIndex < this.chartToCurvesToData.length;
            ++chartIndex
        ) {
            for (
                let curveIndex = 0;
                curveIndex < this.chartToCurvesToData[chartIndex].length;
                ++curveIndex
            ) {
                this.chartToCurvesToData[chartIndex][curveIndex].push({
                    x: timestampDeciseconds,
                    y: chartToCurvesToValue[chartIndex][curveIndex],
                });
            }
        }
    }
}

class Chart {
    name: string;
    inner: chartjs.Chart<"scatter", { x: number; y: number }[], number>;
    collection: Collection;

    constructor(
        canvas: HTMLCanvasElement,
        properties: ChartProperties,
        collection: Collection,
        chartIndex: number,
    ) {
        this.name = properties.name;
        this.inner = new chartjs.Chart(canvas.getContext("2d"), {
            type: "scatter",
            data: {
                datasets: properties.curves.map((curve, index) => ({
                    spanGaps: false,
                    label: curve.name,
                    backgroundColor: curve.color,
                    borderColor: curve.color,
                    borderDash: curve.dash == null ? [] : [5, 5],
                    pointHoverRadius: 0,
                    borderJoinStyle: "round",
                    data: collection.chartToCurvesToData[chartIndex][index],
                    showLine: true,
                    yAxisID: curve.axis === 1 ? "y1" : "y",
                    order: curve.order,
                })),
            },
            options: {
                normalized: true,
                animation: false,
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    x: {
                        display: true,
                        type: "linear",
                        min: 0,
                        max: collection.range,
                        ticks: {
                            maxRotation: 0,
                            stepSize: 10.0,
                            autoSkipPadding: 20,
                            font: {
                                family: '"Roboto Mono", monospace',
                            },
                            sampleSize: 1,
                            color: "#aaaaaa",
                            callback: (value: number) => {
                                const date = new Date(value * 100);
                                return `${date.getUTCSeconds().toString().padStart(2, "0")}.${Math.floor(date.getUTCMilliseconds() / 100).toFixed(0)}`;
                            },
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
                        position: "left",
                        type: properties.yAxes[0].logarithmic
                            ? "logarithmic"
                            : "linear",
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
                            text: properties.yAxes[0].label,
                            color: "#aaaaaa",
                            padding: 10,
                            font: {
                                size: 14,
                            },
                        },
                    },
                    ...(properties.yAxes[1] == null
                        ? {}
                        : {
                              y1: {
                                  display: true,
                                  position: "right",
                                  type: properties.yAxes[1].logarithmic
                                      ? "logarithmic"
                                      : "linear",
                                  ticks: {
                                      maxRotation: 0,
                                      autoSkipPadding: 20,
                                      font: {
                                          family: '"Roboto Mono", monospace',
                                      },
                                      color: "#aaaaaa",
                                  },
                                  grid: {
                                      drawOnChartArea: false,
                                  },
                                  border: {
                                      color: "#aaaaaa",
                                  },
                                  title: {
                                      display: true,
                                      text: properties.yAxes[1].label,
                                      color: "#aaaaaa",
                                      padding: 10,
                                      font: {
                                          size: 14,
                                      },
                                  },
                              },
                          }),
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
                        enabled: false,
                        external: function (context) {
                            const parent = context.chart.canvas.parentNode;
                            let container: HTMLDivElement =
                                parent.querySelector(".container");
                            if (
                                context.tooltip.opacity === 0 ||
                                context.tooltip.dataPoints == null ||
                                context.tooltip.dataPoints.length === 0
                            ) {
                                container.style.opacity = "0";
                                return;
                            }
                            const dataPoints = context.tooltip.dataPoints
                                .slice()
                                .sort(
                                    (a, b) => a.datasetIndex - b.datasetIndex,
                                );
                            if (container == null) {
                                container = document.createElement("div");
                                container.classList.add("container");
                                container.style.position = "absolute";
                                container.style.left = "0";
                                container.style.top = "0";
                                container.style.right = "0";
                                container.style.bottom = "0";
                                container.style.pointerEvents = "none";
                                const line = document.createElement("div");
                                line.classList.add("line");
                                line.style.position = "absolute";
                                line.style.backgroundColor = "#CCCCCC";
                                line.style.width = "1px";
                                container.appendChild(line);
                                const points = document.createElement("div");
                                points.style.position = "absolute";
                                points.style.left = "0";
                                points.style.top = "0";
                                points.style.right = "0";
                                points.style.bottom = "0";
                                points.classList.add("points");
                                for (const dataPoint of dataPoints) {
                                    const point = document.createElement("div");
                                    point.style.position = "absolute";
                                    point.style.width = "8px";
                                    point.style.height = "8px";
                                    point.style.borderRadius = "4px";
                                    // @ts-ignore
                                    point.style.backgroundColor =
                                        dataPoint.dataset.backgroundColor;
                                    point.style.border = "1px solid #CCCCCC";
                                    points.appendChild(point);
                                }
                                container.appendChild(points);
                                const tooltip = document.createElement("div");
                                tooltip.classList.add("tooltip");
                                tooltip.style.userSelect = "none";
                                tooltip.style.position = "absolute";
                                tooltip.style.background = "#000000C0";
                                tooltip.style.borderRadius = "6px";
                                tooltip.style.padding = "6px";
                                tooltip.style.fontFamily =
                                    '"RobotoMono", monospace';
                                tooltip.style.fontSize = "12px";
                                tooltip.style.color = "#FFFFFF";
                                const tooltipTitle =
                                    document.createElement("div");
                                tooltipTitle.classList.add("title");
                                tooltipTitle.style.paddingBottom = "6px";
                                tooltip.appendChild(tooltipTitle);
                                for (const dataPoint of dataPoints) {
                                    const row = document.createElement("div");
                                    row.classList.add("row");
                                    row.style.display = "flex";
                                    row.style.alignItems = "center";
                                    const icon = document.createElement("div");
                                    icon.style.width = "8px";
                                    icon.style.height = "8px";
                                    icon.style.borderRadius = "4px";
                                    icon.style.flexGrow = "0";
                                    icon.style.flexShrink = "0";
                                    // @ts-ignore
                                    icon.style.backgroundColor =
                                        dataPoint.dataset.backgroundColor;
                                    icon.style.border = "1px solid #CCCCCC";
                                    row.appendChild(icon);
                                    const label = document.createElement("div");
                                    const name = document.createElement("span");
                                    name.innerText = dataPoint.dataset.label;
                                    name.style.paddingLeft = "6px";
                                    name.style.color = "#AAAAAA";
                                    label.appendChild(name);
                                    const value =
                                        document.createElement("span");
                                    value.classList.add("value");
                                    value.style.paddingLeft = "12px";
                                    label.appendChild(value);
                                    const yAxis =
                                        dataPoint.dataset.yAxisID === "y1"
                                            ? properties.yAxes[1]
                                            : properties.yAxes[0];
                                    if (yAxis.units != null) {
                                        const units =
                                            document.createElement("span");
                                        units.innerText = yAxis.units;
                                        units.style.paddingLeft = "6px";
                                        label.appendChild(units);
                                    }
                                    row.appendChild(label);
                                    tooltip.appendChild(row);
                                }
                                container.appendChild(tooltip);
                                parent.appendChild(container);
                            }

                            const x = dataPoints[0].element.x;
                            const line: HTMLDivElement =
                                container.querySelector(".line");
                            line.style.top = `${context.chart.chartArea.top}px`;
                            line.style.height = `${context.chart.chartArea.bottom - context.chart.chartArea.top}px`;
                            line.style.left = `${x - 0.5}px`;
                            const points: HTMLDivElement =
                                container.querySelector(".points");
                            let pointIndex = 0;
                            for (const point of points.children) {
                                const dataPoint = dataPoints[pointIndex];
                                (point as HTMLDivElement).style.left =
                                    `${dataPoint.element.x - 4}px`;
                                (point as HTMLDivElement).style.top =
                                    `${dataPoint.element.y - 4}px`;
                                ++pointIndex;
                            }
                            const tooltip: HTMLDivElement =
                                container.querySelector(".tooltip");
                            const tooltipWidth = 185 + 6 * 2;
                            const tooltipHeight =
                                6 * 3 + 14 * (dataPoints.length + 1);
                            tooltip.style.width = `${tooltipWidth}px`;
                            tooltip.style.height = `${tooltipHeight}px`;
                            const middle =
                                (context.chart.chartArea.left +
                                    context.chart.chartArea.right) /
                                2;
                            const leftOverflow = Math.max(
                                tooltipWidth - (x - 6),
                                0,
                            );
                            const rightOverflow = Math.max(
                                x +
                                    6 +
                                    tooltipWidth -
                                    context.chart.chartArea.right,
                                0,
                            );
                            let leftSide: boolean;
                            if (x < middle) {
                                if (rightOverflow === 0) {
                                    leftSide = false;
                                } else {
                                    leftSide = leftOverflow < rightOverflow;
                                }
                            } else {
                                if (leftOverflow === 0) {
                                    leftSide = true;
                                } else {
                                    leftSide = leftOverflow < rightOverflow;
                                }
                            }
                            if (leftSide) {
                                if (leftOverflow === 0) {
                                    tooltip.style.left = `${x - 6 - tooltipWidth}px`;
                                } else {
                                    tooltip.style.left = "0";
                                }
                            } else {
                                if (rightOverflow === 0) {
                                    tooltip.style.left = `${x + 6}px`;
                                } else {
                                    tooltip.style.left = `${context.chart.chartArea.right - tooltipWidth}px`;
                                }
                            }
                            tooltip.style.top = `${(context.chart.chartArea.bottom + context.chart.chartArea.top - tooltipHeight) / 2}px`;
                            const title: HTMLDivElement =
                                container.querySelector(".title");
                            title.innerText =
                                collection.extendedLabels[
                                    dataPoints[0].dataIndex
                                ];
                            pointIndex = 0;
                            for (const value of container.querySelectorAll(
                                ".value",
                            )) {
                                const y: number =
                                    dataPoints[pointIndex].raw["y"];
                                if (y === 0) {
                                    (value as HTMLDivElement).innerText = "0";
                                } else {
                                    const digits = Math.max(
                                        3 - Math.floor(Math.log10(y)),
                                        0,
                                    );
                                    (value as HTMLDivElement).innerText =
                                        y.toLocaleString("en", {
                                            minimumFractionDigits: digits,
                                            maximumFractionDigits: digits,
                                        });
                                }
                                ++pointIndex;
                            }
                            container.style.opacity = "1";
                        },
                    },
                },
            },
            plugins: [
                {
                    id: "tooltip-hidder",
                    afterEvent: (chart, args) => {
                        if (!args.inChartArea) {
                            chart.tooltip.setActiveElements([], {
                                x: 0,
                                y: 0,
                            });
                        }
                    },
                },
            ],
        });
        this.collection = collection;
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
            const expected = this.chartsProperties[chartIndex].curves.length;
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
        for (const [context, id] of this.contextsAndIds) {
            for (const chart of context.charts) {
                chart.inner.options.scales.x.min =
                    chart.collection.timestamps[
                        chart.collection.timestamps.length - 1
                    ] - chart.collection.range;
                chart.inner.options.scales.x.max =
                    chart.collection.timestamps[
                        chart.collection.timestamps.length - 1
                    ];
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
