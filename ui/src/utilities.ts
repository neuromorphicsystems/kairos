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

let unique = 0;
export function nextUnique(): number {
    ++unique;
    return unique;
}
