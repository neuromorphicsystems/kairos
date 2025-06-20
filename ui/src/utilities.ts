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

export function size(bytes: number): string {
    const gigabytes = bytes / 1e9;
    if (gigabytes >= 100.0) {
        return `${gigabytes.toFixed(0)} GB`;
    }
    if (gigabytes >= 10.0) {
        `${gigabytes.toFixed(1)} GB`;
    }
    return `${gigabytes.toFixed(2)} GB`;
}

let unique = 0;
export function nextUnique(): number {
    ++unique;
    return unique;
}

export function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(Math.max(value, minimum), maximum);
}
