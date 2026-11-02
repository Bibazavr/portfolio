//! Builds the static part of the site into `dist/`, or into the directory given
//! as the first argument: the classic résumé in every language, the 404 page,
//! the sitemap and robots.txt, plus everything in `static/`. Run it after
//! trunk has built the plaza into `dist/plaza/`, so `/` can load the plaza.

mod pages;

use std::{
    env,
    error::Error,
    fs,
    hash::{DefaultHasher, Hasher},
    io,
    path::{Path, PathBuf},
};

use askama::Template;
use content::{Lang, Stats};

use pages::{ClassicPage, NotFoundPage, PlazaShell, RedirectPage};

/// One plaza build per renderer, in `dist/plaza/<renderer>/`.
const RENDERERS: [&str; 2] = ["webgl2", "webgpu"];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives in <root>/crates/classic");
    let out = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("dist"));
    let portfolio = content::load()?;
    let site_url = portfolio.site.url.as_str();

    let static_dir = root.join("static");
    copy_dir(&static_dir, &out)?;

    let cv = static_dir
        .join(&portfolio.site.cv)
        .is_file()
        .then_some(portfolio.site.cv.as_str());
    if cv.is_none() {
        eprintln!(
            "warning: static/{} is missing, so the CV link is hidden",
            portfolio.site.cv
        );
    }

    let og_image = static_dir
        .join(&portfolio.site.og_image)
        .is_file()
        .then(|| format!("{site_url}/{}", portfolio.site.og_image));
    if og_image.is_none() {
        eprintln!(
            "warning: static/{} is missing, so pages have no preview image",
            portfolio.site.og_image
        );
    }

    let stats = Stats::read(&root.join("content/stats.json")).unwrap_or_else(|e| {
        eprintln!("warning: {e}; leaving the stats out");
        None
    });

    let builds = plaza_builds(&out)?;
    let plaza = !builds.is_empty();

    for lang in Lang::ALL {
        let dir = out.join(pages::classic_path(lang).trim_start_matches('/'));
        write(
            &dir.join("index.html"),
            &ClassicPage::new(
                &portfolio,
                lang,
                cv,
                plaza,
                stats.as_ref(),
                og_image.as_deref(),
            )
            .render()?,
        )?;
    }
    write(
        &out.join("404.html"),
        &NotFoundPage::new(&portfolio).render()?,
    )?;
    let home = if plaza {
        let version = build_version(&out, &builds)?;
        PlazaShell::new(
            &portfolio,
            builds,
            version,
            cv,
            stats.as_ref(),
            og_image.as_deref(),
        )?
        .render()?
    } else {
        eprintln!("warning: no plaza build in dist/plaza/, so / forwards to the classic page");
        RedirectPage::new(&portfolio).render()?
    };
    write(&out.join("index.html"), &home)?;
    write(&out.join("sitemap.xml"), &pages::sitemap(site_url, plaza))?;
    write(
        &out.join("robots.txt"),
        &format!("User-agent: *\nAllow: /\n\nSitemap: {site_url}/sitemap.xml\n"),
    )?;

    println!("Built {}", out.display());
    Ok(())
}

/// Finds the plaza builds, with the size of each one's WASM, and removes what
/// trunk leaves next to them: its `index.html` is a dev page, and the shell
/// page loads the builds instead.
fn plaza_builds(out: &Path) -> io::Result<Vec<(&'static str, u64)>> {
    let mut builds = Vec::new();
    for renderer in RENDERERS {
        let dir = out.join("plaza").join(renderer);
        if !dir.join("plaza_bg.wasm").is_file() {
            continue;
        }
        for leftover in ["index.html", ".trunk-lock"] {
            match fs::remove_file(dir.join(leftover)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        builds.push((renderer, fs::metadata(dir.join("plaza_bg.wasm"))?.len()));
    }
    Ok(builds)
}

/// A hash of the build files. GitHub Pages caches for 10 minutes and trunk's
/// names don't change, so the shell adds this to the URLs it loads.
fn build_version(out: &Path, builds: &[(&str, u64)]) -> io::Result<String> {
    let mut hasher = DefaultHasher::new();
    for (renderer, _) in builds {
        for file in ["plaza.js", "plaza_bg.wasm"] {
            hasher.write(&fs::read(out.join("plaza").join(renderer).join(file))?);
        }
    }
    Ok(format!("{:016x}", hasher.finish()))
}

fn write(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, contents)
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
