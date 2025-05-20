```sh
cd ui/extension
cargo build --target wasm32-unknown-unknown
rm -rf build
wasm-bindgen target/wasm32-unknown-unknown/debug/extension.wasm --out-dir build --target web
```
