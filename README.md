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
