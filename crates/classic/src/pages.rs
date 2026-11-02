use askama::Template;
use content::{
    CareerStop, Education, Job, Lang, Link, LinkKind, Month, Portfolio, Project, ProjectKind, Repo,
    Spot, Text,
};
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
    fmt: Fmt<'a>,
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
            fmt: Fmt::new(p, lang),
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
}

/// Dates and labels shared by the classic page and the plaza's panels.
#[derive(Clone, Copy)]
pub struct Fmt<'a> {
    p: &'a Portfolio,
    lang: Lang,
}

impl<'a> Fmt<'a> {
    pub fn new(p: &'a Portfolio, lang: Lang) -> Self {
        Self { p, lang }
    }

    fn job_period(&self, job: &Job) -> String {
        self.period(job.start, job.end)
    }

    fn education_period(&self, school: &Education) -> String {
        self.period(school.start, Some(school.end))
    }

    fn stop_period(&self, stop: &CareerStop) -> String {
        match stop.project {
            Some(project) => self
                .project_period(stop.job, project)
                .unwrap_or_else(|| self.job_period(stop.job)),
            None => self.job_period(stop.job),
        }
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

/// The plaza's panels in one language: one per spot, shown by the shell when
/// Bevy reports the skater is in that spot.
#[derive(Template)]
#[template(path = "panels.html")]
pub struct Panels<'a> {
    p: &'a Portfolio,
    lang: Lang,
    cv: Option<&'a str>,
    fmt: Fmt<'a>,
}

impl<'a> Panels<'a> {
    pub fn new(p: &'a Portfolio, lang: Lang, cv: Option<&'a str>) -> Self {
        Self {
            p,
            lang,
            cv,
            fmt: Fmt::new(p, lang),
        }
    }

    fn t(&self, text: &'a Text) -> &'a str {
        text.get(self.lang)
    }

    fn code(&self) -> &'static str {
        self.lang.code()
    }

    fn id(&self, spot: Spot) -> String {
        spot.id()
    }

    fn career(&self) -> Vec<(String, CareerStop<'a>)> {
        let stops = self.p.career().into_iter().enumerate();
        stops
            .map(|(i, stop)| (Spot::Career(i).id(), stop))
            .collect()
    }

    fn rails(&self) -> Vec<(String, &'a Repo)> {
        let repos = self.p.open_source.iter().enumerate();
        repos.map(|(i, repo)| (Spot::Rail(i).id(), repo)).collect()
    }

    // The content crate's tests check these exist.
    fn leetcode(&self) -> &'a Link {
        self.p
            .identity
            .link(LinkKind::LeetCode)
            .expect("LeetCode link")
    }

    fn telegram(&self) -> &'a Link {
        self.p
            .identity
            .link(LinkKind::Telegram)
            .expect("Telegram link")
    }

    fn stairs_repo(&self) -> &'a Repo {
        self.p.stairs_repo().expect("plaza.stairs_repo")
    }
}

/// The plaza's page at `/`: device check, loader, the panels, the touch stick
/// and the always-visible link to the classic page.
#[derive(Template)]
#[template(path = "plaza.html")]
pub struct PlazaShell<'a> {
    p: &'a Portfolio,
    /// Renderers with a build in `dist/plaza/<renderer>/`.
    builds: Vec<&'static str>,
    /// Cache-busting query for the build files, whose names don't change.
    version: String,
    /// Rendered [`Panels`] in every language.
    panels: String,
}

impl<'a> PlazaShell<'a> {
    pub fn new(
        p: &'a Portfolio,
        builds: Vec<&'static str>,
        version: String,
        cv: Option<&'a str>,
    ) -> askama::Result<Self> {
        let panels = Lang::ALL
            .iter()
            .map(|&lang| Panels::new(p, lang, cv).render())
            .collect::<askama::Result<String>>()?;
        Ok(Self {
            p,
            builds,
            version,
            panels,
        })
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

    /// `{ "en": { "ollie": "Ollie", … }, "ru": { … } }` for the score counter.
    fn tricks_json(&self) -> String {
        let names: serde_json::Map<String, serde_json::Value> = Lang::ALL
            .iter()
            .map(|&lang| {
                let tricks = self
                    .p
                    .plaza
                    .tricks
                    .all()
                    .map(|(id, name)| (id.to_owned(), json!(name.get(lang))));
                (
                    lang.code().to_owned(),
                    serde_json::Value::Object(tricks.into_iter().collect()),
                )
            })
            .collect();
        // Keep a "</" inside a string from closing the <script> element.
        serde_json::Value::Object(names)
            .to_string()
            .replace("</", "<\\/")
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
