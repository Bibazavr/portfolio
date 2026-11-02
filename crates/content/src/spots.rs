//! The plaza's spots. Each one is a trigger zone in the plaza and a panel on
//! the page, and both sides name the panel with [`Spot::id`].

use crate::{Job, Lang, Month, Portfolio, Project, Repo};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spot {
    /// Identity, summary and links, where the skater starts.
    Start,
    /// A kicker ramp on the career line; indexes [`Portfolio::career`].
    Career(usize),
    /// The big gap jump: the highlights.
    Gap,
    /// A rail in the rail garden; indexes `open_source`.
    Rail(usize),
    SkillShop,
    /// LeetCode, and the site being written in Rust.
    Stairs,
    /// The apps.
    Billboard,
    /// Contact and CV.
    PhoneBooth,
    /// The hidden Bibacode sticker.
    Sticker,
}

impl Spot {
    pub fn id(self) -> String {
        match self {
            Spot::Start => "start".into(),
            Spot::Career(i) => format!("career-{i}"),
            Spot::Gap => "gap".into(),
            Spot::Rail(i) => format!("rail-{i}"),
            Spot::SkillShop => "skills".into(),
            Spot::Stairs => "stairs".into(),
            Spot::Billboard => "billboard".into(),
            Spot::PhoneBooth => "contact".into(),
            Spot::Sticker => "sticker".into(),
        }
    }
}

/// A stop on the career line: one project, or a job that lists none.
#[derive(Clone, Copy, Debug)]
pub struct CareerStop<'a> {
    pub job: &'a Job,
    pub project: Option<&'a Project>,
}

impl CareerStop<'_> {
    /// A project without dates sits at its job's start.
    pub fn start(&self) -> Month {
        self.project
            .and_then(|project| project.start)
            .unwrap_or(self.job.start)
    }

    /// The project's dates, or its job's when it has none. A project without
    /// an end date ran until the job ended.
    pub fn period(&self, portfolio: &Portfolio, lang: Lang) -> String {
        match self
            .project
            .and_then(|project| Some((project.start?, project.end)))
        {
            Some((start, end)) => portfolio.period(lang, start, end.or(self.job.end)),
            None => portfolio.period(lang, self.job.start, self.job.end),
        }
    }
}

impl Portfolio {
    /// "Oct 2025 – present", "2025 – 2026", or just "2026".
    pub fn period(&self, lang: Lang, start: Month, end: Option<Month>) -> String {
        let month = |m: Month| match m.month {
            Some(n) => format!("{} {}", self.ui.months.name(n, lang), m.year),
            None => m.year.to_string(),
        };
        match end {
            Some(end) if end == start => month(start),
            Some(end) => format!("{} – {}", month(start), month(end)),
            None => format!("{} – {}", month(start), self.ui.present.get(lang)),
        }
    }

    /// The career line, oldest first.
    pub fn career(&self) -> Vec<CareerStop<'_>> {
        let mut stops: Vec<_> = self
            .experience
            .iter()
            .flat_map(|job| {
                let projects = job.projects.iter().map(Some);
                let stops: Vec<_> = if job.projects.is_empty() {
                    vec![None]
                } else {
                    projects.collect()
                };
                stops
                    .into_iter()
                    .map(move |project| CareerStop { job, project })
            })
            .collect();
        stops.sort_by_key(CareerStop::start);
        stops
    }

    /// Every spot on the plaza.
    pub fn spots(&self) -> Vec<Spot> {
        let mut spots = vec![
            Spot::Start,
            Spot::Gap,
            Spot::SkillShop,
            Spot::Stairs,
            Spot::Billboard,
            Spot::PhoneBooth,
            Spot::Sticker,
        ];
        spots.extend((0..self.career().len()).map(Spot::Career));
        spots.extend((0..self.open_source.len()).map(Spot::Rail));
        spots
    }

    pub fn stairs_repo(&self) -> Option<&Repo> {
        self.open_source
            .iter()
            .find(|repo| repo.name == self.plaza.stairs_repo)
    }
}
