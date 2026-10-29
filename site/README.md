# Project page

`index.html` is the project's page on GitHub Pages: one static file, inline CSS, no JavaScript. It also loads Literata from Google Fonts (falls back to Georgia).

The screenshots are not stored here. `.github/workflows/pages.yml` copies them at build time into `_site/screenshots/` from `docs/screenshots/readme/` when that folder exists, otherwise from `docs/screenshots/v1.15/` (dropping the `v1.15-` prefix: `v1.15-write.png` becomes `write.png`). It fails if a `screenshots/...` image the page uses is missing.

## Preview locally

From the repository root:

```sh
rm -rf /tmp/sigma-site && mkdir -p /tmp/sigma-site/screenshots
cp site/index.html /tmp/sigma-site/
for f in docs/screenshots/v1.15/v1.15-*.png; do b=$(basename "$f"); cp "$f" "/tmp/sigma-site/screenshots/${b#v1.15-}"; done
cd /tmp/sigma-site && python3 -m http.server 8000   # then open http://localhost:8000
```
