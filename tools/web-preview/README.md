# Web preview of `sigma-render` (P3-06)

A thin shell that draws one fixture note with the app's renderer, as a
`wasm32-unknown-unknown` module and a small static HTML page. It is a
pilot, not a product, and it is not published.

The `kobo` feature stays off. Paths are unused: the fixture is compiled
in. There is no editor, drawer, plugins, sync, or handwriting.

## Build and open the page

From the repository root (needs the `wasm32-unknown-unknown` target):

```sh
cargo build --locked --release --target wasm32-unknown-unknown -p sigma-web-preview
cp target/wasm32-unknown-unknown/release/sigma_web_preview.wasm tools/web-preview/
cd tools/web-preview && python3 -m http.server 8000
```

Then open <http://localhost:8000>. The page fetches `sigma_web_preview.wasm`
from the same directory; do not commit that file.

## CI smoke

The `preview:smoke` job builds the module and runs `preview_smoke` in a
pinned Wasmtime. It is a pipeline gate, not a release or Pages deploy.

Locally, after building the wasm as above and with `wasmtime` on `PATH`:

```sh
tools/web-preview/ci-smoke.sh target/wasm32-unknown-unknown/release/sigma_web_preview.wasm
```
