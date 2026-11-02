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
| `crates/plaza/assets/models` | Models from Kenney's Mini Skate pack (CC0), with the palette texture recoloured to the studio purple. They're embedded in the WASM. |
| `crates/classic` | Generates the classic page, the plaza's shell page at `/`, 404, sitemap and robots.txt. |
| `static/` | Copied to the site root as-is: CNAME, favicon, CV, images. |
| `build.sh` | Builds everything into `dist/`; CI runs the same steps. |
| `.github/workflows/deploy.yml` | Builds both plaza renderers in parallel, then the site, and deploys to GitHub Pages. |

## How `/` loads the plaza

Bevy picks its renderer at compile time, so the plaza is built twice: `dist/plaza/webgl2/` and
`dist/plaza/webgpu/`. The shell page checks the device, uses the WebGPU build when the browser has
WebGPU, and falls back to WebGL2 if WebGPU fails while starting. `?renderer=webgl2` or
`?renderer=webgpu` forces one, and `?lang=ru` forces the language.

## Spots and panels

Each portfolio spot in the plaza (`crates/plaza/src/spots.rs`) is a trigger zone marked by a
purple pad. Bevy reports the spot the skater is in through `window.plazaBridge`, and the page shows
that spot's panel. The panels are plain HTML generated from `portfolio.ron` at build time
(`crates/classic/templates/panels.html`), in both languages, so links, focus, Esc and screen
readers work as usual. Spot and panel names come from `content::Spot`, so both sides agree.
The touch stick is HTML too and passes its position to Bevy through the WASM export `set_stick`.

## Live stats

`content/stats.json` holds GitHub stars, forks and language for the `open_source` repos and
LeetCode solved counts (plus contest rating, once there is one). `scripts/update-stats.mjs`
(Node, no dependencies) refreshes it; `.github/workflows/stats.yml` runs it nightly and on demand,
commits only when the numbers changed, and then calls the deploy workflow, since pushes made with
`GITHUB_TOKEN` don't trigger other workflows. A failed fetch keeps the previous numbers. The script
only reads `portfolio.ron`. Both views show whatever stats are there and leave out the rest.

```bash
node scripts/update-stats.mjs
```

## Skating

WASD or arrows to skate and Space to ollie; on touch screens, the stick and the ollie button.
The board rolls up ramps and back down fakie, gets air out of the halfpipe (turning around in the
air), and locks onto a rail when it comes down on one. Ollies, airs, gaps and grinds score points.
Sound is synthesised in the page with Web Audio, so there are no audio files; it starts after the
first tap or key press, as browsers require, and the speaker button mutes it.

## Size and loading

Release builds are the plaza's WASM: `opt-level = "z"`, LTO, one codegen unit and stripped
symbols, then `wasm-opt -Oz` (see `crates/plaza/index.html`), with only the Bevy features the plaza
uses and info/debug logging compiled out. Each renderer's build is about 19 MB, 5.7 MB gzipped.
`build.sh` remaps local paths out of the binary.

The page downloads the WASM itself to show real progress (the shell knows each build's size), then
bevy_asset_loader loads the models before the plaza is built, and Bevy tells the page when it has
drawn its first frames so the loading screen can go.

## Social preview

`static/og.jpg` is a 1200×630 shot of the plaza, used for `og:image` on every page. To redo it after
changing the plaza:

```bash
cargo run -p plaza -- --og-image /tmp/og.png
```

```bash
sips -s format jpeg -s formatOptions 82 /tmp/og.png --out static/og.jpg
```

## Analytics

Cloudflare Web Analytics (cookie-free, no banner). The beacon token is `site.cloudflare_analytics`
in `portfolio.ron`; remove it to turn analytics off.

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

## Credits

3D models: [Mini Skate](https://kenney.nl/assets/mini-skate) by Kenney, CC0.

## Deploy setup

1. In the repo settings, set Pages → Source to **GitHub Actions**.
2. Set Pages → Custom domain to `me.bibazavr.fyi` and enable HTTPS. Actions deployments
   ignore the `CNAME` file, so the setting is what counts.
3. DNS: `me` CNAME → `bibazavr.github.io`.
