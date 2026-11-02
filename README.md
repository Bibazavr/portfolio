# me.bibazavr.fyi

Portfolio of Kirill Lebedev (Bibazavr), in two views built from one content file:

- `/` will be the interactive skate plaza (Bevy, WASM). Until then it forwards to `/classic/`.
- `/classic/` (EN) and `/classic/ru/` (RU) are the plain HTML résumé: fast, accessible and crawlable.

## Layout

| Path | What it is |
|---|---|
| `content/portfolio.ron` | Everything a visitor reads, in EN and RU. Edit content here only. |
| `crates/content` | Serde model for `portfolio.ron`, shared by both views. Its tests check the file parses. |
| `crates/classic` | Generates the classic page, 404, sitemap and robots.txt with Askama templates. |
| `static/` | Copied to the site root as-is: CNAME, favicon, CV, images. |
| `.github/workflows/deploy.yml` | Tests, builds `dist/` and deploys it to GitHub Pages. |

## Run locally

```bash
cargo test
cargo run -p classic
python3 -m http.server 8080 --directory dist
```

Then open http://localhost:8080/classic/.

## Editing content

Text is either a plain string, shown as-is in every language, or `(en: "...", ru: "...")`.
Dates are `"YYYY-MM"`, or `"YYYY"` when the month isn't known. A job or project
without an `end` is ongoing; a project with a `start` and no `end` ran until its job ended.

## Deploy setup

1. In the repo settings, set Pages → Source to **GitHub Actions**.
2. Set Pages → Custom domain to `me.bibazavr.fyi` and enable HTTPS. Actions deployments
   ignore the `CNAME` file, so the setting is what counts.
3. DNS: `me` CNAME → `bibazavr.github.io`.
