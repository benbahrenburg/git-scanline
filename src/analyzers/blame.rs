use std::collections::{HashMap, HashSet};
use crate::types::{Commit, SiloData};

/// Analyzes author concentration per file using commit history.
/// High single-author ratio = knowledge silo = elevated bug risk.
pub fn analyze_authors(commits: &[Commit], files: &[String]) -> HashMap<String, SiloData> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    // filename → author → commit count
    let mut file_authors: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for commit in commits {
        for file in &commit.files {
            if !file_set.contains(file.as_str()) { continue; }
            *file_authors.entry(file.clone()).or_default()
                .entry(commit.author.clone()).or_insert(0) += 1;
        }
    }

    files.iter().map(|file| {
        let author_map = file_authors.get(file);

        let data = match author_map {
            None => SiloData {
                top_author: "unknown".to_string(),
                top_author_percent: 100.0,
                author_count: 1,
            },
            Some(m) if m.is_empty() => SiloData {
                top_author: "unknown".to_string(),
                top_author_percent: 100.0,
                author_count: 1,
            },
            Some(m) => {
                let total: usize = m.values().sum();
                let (top_author, top_count) = m.iter()
                    .max_by_key(|(_, &v)| v)
                    .map(|(k, &v)| (k.as_str(), v))
                    .unwrap_or(("unknown", 0));

                SiloData {
                    top_author: top_author.to_string(),
                    top_author_percent: if total > 0 {
                        (top_count as f64 / total as f64) * 100.0
                    } else { 100.0 },
                    author_count: m.len(),
                }
            }
        };

        (file.clone(), data)
    }).collect()
}
