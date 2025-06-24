export function utcString(): string {
    const date = new Date();
    return `${date.getUTCFullYear().toString().padStart(4, "0")}-${(
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
        .padStart(2, "0")}:${date
        .getUTCSeconds()
        .toString()
        .padStart(2, "0")}.${date
        .getUTCMilliseconds()
        .toString()
        .padStart(3, "0")}`;
}

export function gbSizeToString(bytes: number): string {
    const gigabytes = bytes / 1e9;
    if (gigabytes >= 100.0) {
        return `${gigabytes.toFixed(0)} GB`;
    }
    if (gigabytes >= 10.0) {
        `${gigabytes.toFixed(1)} GB`;
    }
    return `${gigabytes.toFixed(2)} GB`;
}

export function sizeToString(bytes: bigint): string {
    if (bytes < 1000n) {
        return `${Number(bytes).toFixed(0)} B`;
    }
    if (bytes < 1000000n) {
        return `${(Number(bytes) / 1000).toFixed(2)} kB`;
    }
    if (bytes < 1000000000n) {
        return `${(Number(bytes) / 1000000).toFixed(2)} MB`;
    }
    if (bytes < 1000000000000n) {
        return `${(Number(bytes) / 1000000000).toFixed(2)} GB`;
    }
    return `${(Number(bytes) / 1000000000000).toFixed(2)} TB`;
}

export function durationToString(microseconds: bigint) {
    let deciseconds = microseconds / 100000n;
    const hours = deciseconds / 36000n;
    deciseconds -= hours * 36000n;
    const minutes = deciseconds / 600n;
    deciseconds -= minutes * 600n;
    const seconds = deciseconds / 10n;
    deciseconds -= seconds * 10n;
    return `${Number(hours).toFixed(0).padStart(2, "0")}:${Number(minutes)
        .toFixed(0)
        .padStart(2, "0")}:${Number(seconds)
        .toFixed(0)
        .padStart(2, "0")}.${Number(deciseconds).toFixed(0)}`;
}

let unique = 0;
export function nextUnique(): number {
    ++unique;
    return unique;
}

export function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(Math.max(value, minimum), maximum);
}
