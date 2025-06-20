# Local development

1. Install rust (https://rustup.rs) and node (https://formulae.brew.sh/formula/node).

2. Install the WASM bindings generation tool.

    ```sh
    cargo install wasm-bindgen-cli
    rustup target add wasm32-unknown-unknown
    ```

3. Build the interface (CTRL-C to stop watching for changes).

    ```sh
    cd ui
    npm install
    npm run watch
    ```

4. Open a second terminal to compile and run the server (CTRL-C to stop it).

    ```sh
    cargo run --release
    ```

5. Open _./ui/build/index.html_ in a browser (the browser must support WebTransport, see https://caniuse.com/webtransport for compatibility).

6. After making changes...
    - ...to a file in _./src_, stop the server and run `cargo run --release` again.
    - ...to a file in _./ui/src_, refresh the browser window (`npm run watch` automatically rebuilds _./ui/build/index.html_ when it detects source changes).
    - ...to a file in _./ui/extension_, stop the watcher and run `npm run watch` again (extension files are only compiled when the watcher starts).

# Format specification

## Events (.raw.kai)

| Byte       | Hex value | ASCII | Description                            |
| ---------- | --------- | ----- | -------------------------------------- |
| 0          | `0x4B`    | `K`   | Magic number                           |
| 1          | `0x41`    | `A`   | Magic number                           |
| 2          | `0x49`    | `I`   | Magic number                           |
| 3          | `0x52`    | `R`   | Magic number                           |
| 4          | `0x4F`    | `O`   | Magic number                           |
| 5          | `0x53`    | `S`   | Magic number                           |
| 6          | `0x2D`    | `-`   | Magic number                           |
| 7          | `0x52`    | `R`   | Magic number                           |
| 8          | `0x41`    | `A`   | Magic number                           |
| 9          | `0x57`    | `W`   | Magic number                           |
| 10         | `0x00`    |       | Version number                         |
| 11         | `0x00`    |       | Format id (`0` is Prophesee Gen4 EVT3) |
| 12         | [U0]      |       | Decoder state length (4 bytes LE)      |
| 13         | [U1]      |       | Decoder state length (4 bytes LE)      |
| 14         | [U2]      |       | Decoder state length (4 bytes LE)      |
| 15         | [U3]      |       | Decoder state length (4 bytes LE)      |
| 16..16 + U |           |       | Decoder state                          |

The rest of the file contains raw EVT3 data.

## USB packets timings (.index.kai)

| Byte | Hex value | ASCII | Description                            |
| ---- | --------- | ----- | -------------------------------------- |
| 0    | `0x4B`    | `K`   | Magic number                           |
| 1    | `0x41`    | `A`   | Magic number                           |
| 2    | `0x49`    | `I`   | Magic number                           |
| 3    | `0x52`    | `R`   | Magic number                           |
| 4    | `0x4F`    | `O`   | Magic number                           |
| 5    | `0x53`    | `S`   | Magic number                           |
| 6    | `0x2D`    | `-`   | Magic number                           |
| 7    | `0x49`    | `I`   | Magic number                           |
| 8    | `0x4E`    | `N`   | Magic number                           |
| 9    | `0x44`    | `D`   | Magic number                           |
| 10   | `0x45`    | `E`   | Magic number                           |
| 11   | `0x58`    | `X`   | Magic number                           |
| 12   | `0x00`    |       | Version number                         |
| 13   | `0x00`    |       | Format id (`0` is Prophesee Gen4 EVT3) |

The rest of the file contains USB packets entries (packet offset in .raw.kai, computer timestamp, UTC time, and decoder state).

## Samples (.samples.kai)

| Byte | Hex value | ASCII | Description                       |
| ---- | --------- | ----- | --------------------------------- |
| 0    | `0x4B`    | `K`   | Magic number                      |
| 1    | `0x41`    | `A`   | Magic number                      |
| 2    | `0x49`    | `I`   | Magic number                      |
| 3    | `0x52`    | `R`   | Magic number                      |
| 4    | `0x4F`    | `O`   | Magic number                      |
| 5    | `0x53`    | `S`   | Magic number                      |
| 6    | `0x2D`    | `-`   | Magic number                      |
| 7    | `0x53`    | `S`   | Magic number                      |
| 8    | `0x41`    | `A`   | Magic number                      |
| 9    | `0x4D`    | `M`   | Magic number                      |
| 10   | `0x50`    | `P`   | Magic number                      |
| 11   | `0x4C`    | `L`   | Magic number                      |
| 11   | `0x45`    | `E`   | Magic number                      |
| 11   | `0x53`    | `S`   | Magic number                      |
| 12   | `0x00`    |       | Version number                    |
| 13   | `0x00`    |       | Format id (`0` is Prophesee Gen4) |

The rest of the file contains timestamped samples (illuminance and temperature).
