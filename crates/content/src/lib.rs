//! Everything a visitor reads, deserialized from `content/portfolio.ron`.
//!
//! Both views use this crate: the classic page generator renders it to HTML at
//! build time, and the plaza embeds the same file into its WASM binary.

mod spots;
mod stats;

use std::fmt;

use serde::Deserialize;

pub use spots::{CareerStop, Spot};
pub use stats::{LeetCodeStats, RepoStats, Solved, Stats};

/// The raw RON source, embedded at compile time.
pub const SOURCE: &str = include_str!("../../../content/portfolio.ron");

/// Parses the embedded `portfolio.ron`.
pub fn load() -> Result<Portfolio, ron::error::SpannedError> {
    ron::from_str(SOURCE)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Ru];

    /// BCP 47 code, for `lang` and `hreflang` attributes.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
        }
    }

    /// The language the toggle switches to.
    pub fn other(self) -> Lang {
        match self {
            Lang::En => Lang::Ru,
            Lang::Ru => Lang::En,
        }
    }
}

/// Visitor-facing text. A plain string reads the same in every language;
/// `(en: "...", ru: "...")` gives one string per language.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "RawText")]
pub enum Text {
    Same(String),
    Translated { en: String, ru: String },
}

impl Text {
    pub fn get(&self, lang: Lang) -> &str {
        match (self, lang) {
            (Text::Same(s), _) => s,
            (Text::Translated { en, .. }, Lang::En) => en,
            (Text::Translated { ru, .. }, Lang::Ru) => ru,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawText {
    Same(String),
    Translated(Pair),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    en: String,
    ru: String,
}

impl TryFrom<RawText> for Text {
    type Error = &'static str;

    fn try_from(raw: RawText) -> Result<Self, Self::Error> {
        let text = match raw {
            RawText::Same(s) => Text::Same(s),
            RawText::Translated(Pair { en, ru }) => Text::Translated { en, ru },
        };
        if Lang::ALL
            .iter()
            .any(|&lang| text.get(lang).trim().is_empty())
        {
            return Err("text must not be empty in any language");
        }
        Ok(text)
    }
}

/// A month like `"2025-10"`, or a bare year like `"2026"` when the month isn't known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct Month {
    pub year: u16,
    pub month: Option<u8>,
}

impl Month {
    /// ISO 8601 form, for `<time datetime>`.
    pub fn iso(self) -> String {
        match self.month {
            Some(m) => format!("{}-{m:02}", self.year),
            None => self.year.to_string(),
        }
    }
}

impl TryFrom<String> for Month {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let bad = || format!("expected \"YYYY\" or \"YYYY-MM\", got {s:?}");
        let (year, month) = match s.split_once('-') {
            Some((y, m)) => (y, Some(m.parse::<u8>().map_err(|_| bad())?)),
            None => (s.as_str(), None),
        };
        let year = year.parse::<u16>().map_err(|_| bad())?;
        if year < 1900 || month.is_some_and(|m| !(1..=12).contains(&m)) {
            return Err(bad());
        }
        Ok(Month { year, month })
    }
}

/// An absolute `https://` URL.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Url(String);

impl Url {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Url {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with("https://") && !s.contains(char::is_whitespace) {
            Ok(Url(s))
        } else {
            Err(format!("expected an https:// URL, got {s:?}"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Portfolio {
    pub site: Site,
    pub identity: Identity,
    pub highlights: Vec<Highlight>,
    /// Newest first.
    pub experience: Vec<Job>,
    pub open_source: Vec<Repo>,
    pub skills: Vec<SkillGroup>,
    pub apps: Vec<App>,
    pub education: Vec<Education>,
    pub ui: Ui,
    pub plaza: PlazaText,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    /// Public origin, without a trailing slash.
    pub url: Url,
    /// CV download, relative to the site root.
    pub cv: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub name: Text,
    pub handle: String,
    pub title: Text,
    pub location: Text,
    pub summary: Text,
    pub email: String,
    pub links: Vec<Link>,
    pub fun_facts: Vec<Fact>,
}

impl Identity {
    pub fn link(&self, kind: LinkKind) -> Option<&Link> {
        self.links.iter().find(|link| link.kind == kind)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub kind: LinkKind,
    pub label: Text,
    pub url: Url,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum LinkKind {
    LinkedIn,
    GitHub,
    LeetCode,
    Telegram,
    Studio,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub label: Text,
    pub value: Text,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Highlight {
    pub title: Text,
    pub body: Text,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub company: Text,
    pub role: Text,
    pub start: Month,
    /// `None` while the job is current.
    pub end: Option<Month>,
    pub location: Text,
    #[serde(default)]
    pub points: Vec<Text>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: Text,
    #[serde(default)]
    pub kind: ProjectKind,
    #[serde(default)]
    pub nda: bool,
    /// `None` when the project has no dates of its own.
    pub start: Option<Month>,
    /// `None` after a `start` means the project ran until the job ended.
    pub end: Option<Month>,
    pub summary: Option<Text>,
    #[serde(default)]
    pub points: Vec<Text>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum ProjectKind {
    /// Done for the employer; shown without a badge.
    #[default]
    Employer,
    Own,
    Client,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    /// `owner/name`.
    pub name: String,
    pub url: Url,
    pub description: Text,
    pub language: String,
    pub demo: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillGroup {
    pub name: Text,
    pub items: Vec<Text>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    pub name: String,
    /// Icon, relative to the site root.
    pub icon: String,
    pub tagline: Text,
    pub description: Text,
    pub features: Vec<Text>,
    pub web: Url,
    pub studio: Url,
    /// Availability note, e.g. while the app isn't in the stores yet.
    pub status: Option<Text>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Education {
    pub school: Text,
    pub degree: Text,
    pub start: Month,
    pub end: Month,
}

/// Short month names, January first.
#[derive(Debug, Deserialize)]
#[serde(try_from = "Vec<Text>")]
pub struct Months([Text; 12]);

impl Months {
    /// Name of a month numbered 1–12.
    pub fn name(&self, month: u8, lang: Lang) -> &str {
        self.0[usize::from(month) - 1].get(lang)
    }
}

impl TryFrom<Vec<Text>> for Months {
    type Error = String;

    fn try_from(names: Vec<Text>) -> Result<Self, Self::Error> {
        let count = names.len();
        let names = names
            .try_into()
            .map_err(|_| format!("expected 12 month names, got {count}"))?;
        Ok(Months(names))
    }
}

/// Interface strings: headings, buttons, labels.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    /// Each language's name in itself, for the language toggle.
    pub language_name: Text,
    pub skip_to_content: Text,
    pub highlights: Text,
    pub experience: Text,
    pub open_source: Text,
    pub skills: Text,
    pub apps: Text,
    pub education: Text,
    pub contact: Text,
    pub fun_facts: Text,
    pub present: Text,
    pub months: Months,
    pub own_product: Text,
    pub client: Text,
    pub nda: Text,
    pub email_me: Text,
    pub download_cv: Text,
    pub demo: Text,
    pub open_web: Text,
    pub studio_page: Text,
    pub footer_note: Text,
    pub not_found_title: Text,
    pub not_found_body: Text,
    pub to_resume: Text,
    /// Link from the classic page to the plaza.
    pub plaza: Text,
    /// Link to a repository's code.
    pub source: Text,
    pub stars: Text,
    pub forks: Text,
    pub solved: Text,
    pub easy: Text,
    pub medium: Text,
    pub hard: Text,
    pub contest_rating: Text,
}

/// Text that only the plaza shows.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlazaText {
    /// Accessible name of the plaza's canvas.
    pub label: Text,
    /// The always-visible link to the classic page.
    pub classic_view: Text,
    pub loading: Text,
    pub needs_js: Text,
    pub hint_keys: Text,
    pub hint_touch: Text,
    pub close: Text,
    pub stairs_title: Text,
    pub stairs_body: Text,
    /// `open_source` name of the repository shown on the stair set.
    pub stairs_repo: String,
    pub egg_title: Text,
    pub egg_body: Text,
    /// The touch ollie button.
    pub ollie: Text,
    pub score: Text,
    /// Labels for the mute button.
    pub sound_off: Text,
    pub sound_on: Text,
    pub tricks: Tricks,
}

/// Names the score counter shows for each trick.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tricks {
    pub ollie: Text,
    pub air: Text,
    pub grind: Text,
    pub gap: Text,
}

impl Tricks {
    /// Each trick's name, keyed by the id the plaza reports it with.
    pub fn all(&self) -> [(&'static str, &Text); 4] {
        [
            ("ollie", &self.ollie),
            ("air", &self.air),
            ("grind", &self.grind),
            ("gap", &self.gap),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_ron_parses() {
        let portfolio = load().unwrap_or_else(|e| panic!("content/portfolio.ron: {e}"));
        let starts: Vec<_> = portfolio.experience.iter().map(|job| job.start).collect();
        assert!(
            starts.is_sorted_by(|a, b| a >= b),
            "experience must be newest first"
        );

        // The plaza's spots need these to exist.
        assert!(
            portfolio.stairs_repo().is_some(),
            "plaza.stairs_repo must name an open_source repo"
        );
        for kind in [LinkKind::LeetCode, LinkKind::Telegram] {
            assert!(
                portfolio.identity.link(kind).is_some(),
                "identity.links needs {kind:?}"
            );
        }
        assert!(
            !portfolio.apps.is_empty(),
            "the billboard shows the first app"
        );
    }

    #[test]
    fn career_is_oldest_first() {
        let portfolio = load().unwrap();
        let starts: Vec<_> = portfolio.career().iter().map(CareerStop::start).collect();
        assert!(starts.is_sorted());
        let stops: usize = portfolio
            .experience
            .iter()
            .map(|job| job.projects.len().max(1))
            .sum();
        assert_eq!(starts.len(), stops);
    }

    #[test]
    fn month_parsing() {
        let month = |s: &str| Month::try_from(s.to_owned());
        assert_eq!(
            month("2025-10"),
            Ok(Month {
                year: 2025,
                month: Some(10)
            })
        );
        assert_eq!(
            month("2026"),
            Ok(Month {
                year: 2026,
                month: None
            })
        );
        assert!(month("2025-13").is_err());
        assert!(month("Oct 2025").is_err());
    }
}
