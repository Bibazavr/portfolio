# me.bibazavr.fyi

Portfolio of Kirill Lebedev (Bibazavr), in two views built from one content file:

- `/` is the interactive skate plaza (Bevy, WASM). Small or low-end devices are sent to the classic page; `/?play` skips that check.
- `/classic/` (EN) and `/classic/ru/` (RU) are the plain HTML résumé: fast, accessible and crawlable.

## Layout

| Path | What it is |
|---|---|
| `content/portfolio.ron` | Everything a visitor reads, in EN and RU. Edit content here only. |
| `crates/content` | Serde model for `portfolio.ron`, shared by both views. Its tests check the file parses. |
| `crates/plaza` | The Bevy plaza. `index.html` there is only a dev page for `trunk serve`. |
| `crates/classic` | Generates the classic page, the plaza's shell page at `/`, 404, sitemap and robots.txt. |
| `static/` | Copied to the site root as-is: CNAME, favicon, CV, images. |
| `build.sh` | Builds everything into `dist/`; CI runs the same steps. |
| `.github/workflows/deploy.yml` | Builds both plaza renderers in parallel, then the site, and deploys to GitHub Pages. |

## How `/` loads the plaza

Bevy picks its renderer at compile time, so the plaza is built twice: `dist/plaza/webgl2/` and
`dist/plaza/webgpu/`. The shell page checks the device, uses the WebGPU build when the browser has
WebGPU, and falls back to WebGL2 if WebGPU fails while starting. `?renderer=webgl2` or
`?renderer=webgpu` forces one, and `?lang=ru` forces the language.

## Run locally

Needs Rust, the `wasm32-unknown-unknown` target and [trunk](https://trunkrs.dev).

```bash
./build.sh
python3 -m http.server 8080 --directory dist
```

Then open http://localhost:8080/. For quick plaza iteration, `trunk serve` in `crates/plaza`
serves the dev page on port 8081, and `cargo run -p plaza` opens a native window.

## Editing content

Text is either a plain string, shown as-is in every language, or `(en: "...", ru: "...")`.
Dates are `"YYYY-MM"`, or `"YYYY"` when the month isn't known. A job or project
without an `end` is ongoing; a project with a `start` and no `end` ran until its job ended.

## Deploy setup

1. In the repo settings, set Pages → Source to **GitHub Actions**.
2. Set Pages → Custom domain to `me.bibazavr.fyi` and enable HTTPS. Actions deployments
   ignore the `CNAME` file, so the setting is what counts.
3. DNS: `me` CNAME → `bibazavr.github.io`.
