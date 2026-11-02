use askama::Template;
use content::{Education, Job, Lang, Month, Portfolio, Project, ProjectKind, Text};
use serde_json::json;

const STYLE: &str = include_str!("../assets/classic.css");

/// Site path of the classic page in each language.
pub fn classic_path(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "/classic/",
        Lang::Ru => "/classic/ru/",
    }
}

fn og_locale(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en_US",
        Lang::Ru => "ru_RU",
    }
}

#[derive(Template)]
#[template(path = "classic.html")]
pub struct ClassicPage<'a> {
    p: &'a Portfolio,
    lang: Lang,
    /// CV path, or `None` when the file isn't in `static/` yet.
    cv: Option<&'a str>,
    /// Whether the plaza was built, so the header can link to it.
    plaza: bool,
    style: &'static str,
    json_ld: String,
}

// Askama passes method arguments by reference, but not consistently, so the
// templates never pass a `Lang`: each page gets helpers for what it needs.
impl<'a> ClassicPage<'a> {
    pub fn new(p: &'a Portfolio, lang: Lang, cv: Option<&'a str>, plaza: bool) -> Self {
        Self {
            p,
            lang,
            cv,
            plaza,
            style: STYLE,
            json_ld: person_json_ld(p, lang),
        }
    }

    fn t(&self, text: &'a Text) -> &'a str {
        text.get(self.lang)
    }

    /// The visitor asked for the plaza, so skip its device check.
    fn plaza_href(&self) -> String {
        format!("/?play&lang={}", self.lang.code())
    }

    fn url(&self, lang: Lang) -> String {
        format!("{}{}", self.p.site.url, classic_path(lang))
    }

    fn canonical(&self) -> String {
        self.url(self.lang)
    }

    fn home(&self) -> &'static str {
        classic_path(self.lang)
    }

    fn switch_path(&self) -> &'static str {
        classic_path(self.lang.other())
    }

    /// The other language's name, written in that language.
    fn switch_label(&self) -> &'a str {
        self.p.ui.language_name.get(self.lang.other())
    }

    fn og_locale(&self) -> &'static str {
        og_locale(self.lang)
    }

    fn og_locale_alternate(&self) -> &'static str {
        og_locale(self.lang.other())
    }

    /// `(hreflang, href)` pairs, `x-default` last.
    fn alternates(&self) -> Vec<(&'static str, String)> {
        let mut links: Vec<_> = Lang::ALL.iter().map(|&l| (l.code(), self.url(l))).collect();
        links.push(("x-default", self.url(Lang::En)));
        links
    }

    fn job_period(&self, job: &Job) -> String {
        self.period(job.start, job.end)
    }

    fn education_period(&self, school: &Education) -> String {
        self.period(school.start, Some(school.end))
    }

    /// A project without an end date ran until the job ended.
    fn project_period(&self, job: &Job, project: &Project) -> Option<String> {
        let start = project.start?;
        Some(self.period(start, project.end.or(job.end)))
    }

    fn badges(&self, project: &Project) -> Vec<&'a str> {
        let ui = &self.p.ui;
        let kind = match project.kind {
            ProjectKind::Employer => None,
            ProjectKind::Own => Some(&ui.own_product),
            ProjectKind::Client => Some(&ui.client),
        };
        kind.into_iter()
            .chain(project.nda.then_some(&ui.nda))
            .map(|text| text.get(self.lang))
            .collect()
    }

    /// "Oct 2025 – present", "2025 – 2026", or just "2026".
    fn period(&self, start: Month, end: Option<Month>) -> String {
        let start_label = self.month(start);
        match end {
            Some(end) if end == start => start_label,
            Some(end) => format!("{start_label} – {}", self.month(end)),
            None => format!("{start_label} – {}", self.p.ui.present.get(self.lang)),
        }
    }

    fn month(&self, m: Month) -> String {
        match m.month {
            Some(n) => format!("{} {}", self.p.ui.months.name(n, self.lang), m.year),
            None => m.year.to_string(),
        }
    }
}

fn person_json_ld(p: &Portfolio, lang: Lang) -> String {
    let id = &p.identity;
    let person = json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "name": id.name.get(lang),
        "alternateName": id.handle,
        "jobTitle": id.title.get(lang),
        "description": id.summary.get(lang),
        "url": format!("{}{}", p.site.url, classic_path(lang)),
        "email": format!("mailto:{}", id.email),
        "homeLocation": { "@type": "Place", "name": id.location.get(lang) },
        "sameAs": id.links.iter().map(|link| link.url.as_str()).collect::<Vec<_>>(),
        "alumniOf": p.education.iter()
            .map(|e| json!({ "@type": "CollegeOrUniversity", "name": e.school.get(lang) }))
            .collect::<Vec<_>>(),
    });
    // Keep a "</" inside a string from closing the <script> element.
    person.to_string().replace("</", "<\\/")
}

/// The site-wide 404, in every language at once since the visitor's is unknown.
#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundPage<'a> {
    p: &'a Portfolio,
    style: &'static str,
}

struct NotFoundBlock<'a> {
    lang: &'static str,
    title: &'a str,
    body: &'a str,
    back: &'a str,
    path: &'static str,
}

impl<'a> NotFoundPage<'a> {
    pub fn new(p: &'a Portfolio) -> Self {
        Self { p, style: STYLE }
    }

    fn en(&self, text: &'a Text) -> &'a str {
        text.get(Lang::En)
    }

    fn blocks(&self) -> Vec<NotFoundBlock<'a>> {
        let ui = &self.p.ui;
        Lang::ALL
            .iter()
            .map(|&lang| NotFoundBlock {
                lang: lang.code(),
                title: ui.not_found_title.get(lang),
                body: ui.not_found_body.get(lang),
                back: ui.to_resume.get(lang),
                path: classic_path(lang),
            })
            .collect()
    }
}

/// The plaza's page at `/`: device check, language, loader and the
/// always-visible link to the classic page.
#[derive(Template)]
#[template(path = "plaza.html")]
pub struct PlazaShell<'a> {
    p: &'a Portfolio,
    /// Renderers with a build in `dist/plaza/<renderer>/`.
    builds: Vec<&'static str>,
    /// Cache-busting query for the build files, whose names don't change.
    version: String,
}

impl<'a> PlazaShell<'a> {
    pub fn new(p: &'a Portfolio, builds: Vec<&'static str>, version: String) -> Self {
        Self { p, builds, version }
    }

    fn en(&self, text: &'a Text) -> &'a str {
        text.get(Lang::En)
    }

    fn ru(&self, text: &'a Text) -> &'a str {
        text.get(Lang::Ru)
    }

    fn classic_path_en(&self) -> &'static str {
        classic_path(Lang::En)
    }

    fn classic_path_ru(&self) -> &'static str {
        classic_path(Lang::Ru)
    }

    fn builds_json(&self) -> String {
        serde_json::to_string(&self.builds).expect("strings serialize")
    }
}

/// Stands in for the plaza at `/` when it hasn't been built.
#[derive(Template)]
#[template(path = "redirect.html")]
pub struct RedirectPage<'a> {
    p: &'a Portfolio,
    target: &'static str,
}

impl<'a> RedirectPage<'a> {
    pub fn new(p: &'a Portfolio) -> Self {
        Self {
            p,
            target: classic_path(Lang::En),
        }
    }

    fn en(&self, text: &'a Text) -> &'a str {
        text.get(Lang::En)
    }
}

pub fn sitemap(site_url: &str, plaza: bool) -> String {
    let alternates: String = Lang::ALL
        .iter()
        .map(|l| {
            format!(
                "\n    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{site_url}{}\"/>",
                l.code(),
                classic_path(*l)
            )
        })
        .collect();
    let mut urls: String = Lang::ALL
        .iter()
        .map(|l| {
            format!(
                "\n  <url>\n    <loc>{site_url}{}</loc>{alternates}\n  </url>",
                classic_path(*l)
            )
        })
        .collect();
    if plaza {
        urls.insert_str(
            0,
            &format!("\n  <url>\n    <loc>{site_url}/</loc>\n  </url>"),
        );
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">{urls}\n</urlset>\n"
    )
}
