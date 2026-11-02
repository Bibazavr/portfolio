//! Builds the static part of the site into `dist/`, or into the directory given
//! as the first argument: the classic résumé in every language, the 404 page,
//! the sitemap and robots.txt, plus everything in `static/`.

mod pages;

use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use askama::Template;
use content::Lang;

use pages::{ClassicPage, NotFoundPage, RedirectPage};

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

    for lang in Lang::ALL {
        let dir = out.join(pages::classic_path(lang).trim_start_matches('/'));
        write(
            &dir.join("index.html"),
            &ClassicPage::new(&portfolio, lang, cv).render()?,
        )?;
    }
    write(
        &out.join("404.html"),
        &NotFoundPage::new(&portfolio).render()?,
    )?;
    // Until the plaza exists, `/` forwards to the classic page.
    write(
        &out.join("index.html"),
        &RedirectPage::new(&portfolio).render()?,
    )?;
    write(&out.join("sitemap.xml"), &pages::sitemap(site_url))?;
    write(
        &out.join("robots.txt"),
        &format!("User-agent: *\nAllow: /\n\nSitemap: {site_url}/sitemap.xml\n"),
    )?;

    println!("Built {}", out.display());
    Ok(())
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
