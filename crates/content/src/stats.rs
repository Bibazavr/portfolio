//! Live numbers from `content/stats.json`, which a monthly job refreshes (see
//! `scripts/update-stats.mjs`). Everything is optional: the views show what's
//! there and leave out the rest.

use std::{collections::HashMap, fs, io, path::Path};

use serde::Deserialize;

use crate::Repo;

#[derive(Debug, Default, Deserialize)]
pub struct Stats {
    /// By `owner/name`, as in `open_source`.
    #[serde(default)]
    repos: HashMap<String, RepoStats>,
    pub leetcode: Option<LeetCodeStats>,
}

#[derive(Debug, Deserialize)]
pub struct RepoStats {
    pub stars: Option<u32>,
    pub forks: Option<u32>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeetCodeStats {
    pub solved: Solved,
    pub contest_rating: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Solved {
    pub all: u32,
    pub easy: u32,
    pub medium: u32,
    pub hard: u32,
}

impl Stats {
    /// Reads the stats file; `Ok(None)` when there isn't one.
    pub fn read(path: &Path) -> Result<Option<Self>, String> {
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)
                .map(Some)
                .map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn repo(&self, repo: &Repo) -> Option<&RepoStats> {
        self.repos.get(&repo.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_stats_parse() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/stats.json");
        Stats::read(&path).unwrap_or_else(|e| panic!("{e}"));
    }
}
