use crate::types::{Commit, RevertData};
use std::collections::{HashMap, HashSet};

static REVERT_PATTERN: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"(?i)^revert\b").unwrap());

/// Detects files appearing in revert commits — a strong signal of introduced bugs.
pub fn analyze_reverts(commits: &[Commit], files: &[String]) -> HashMap<String, RevertData> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    let mut file_reverts: HashMap<String, usize> = HashMap::new();

    for commit in commits {
        if !REVERT_PATTERN.is_match(commit.subject.trim()) {
            continue;
        }
        for file in &commit.files {
            if !file_set.contains(file.as_str()) {
                continue;
            }
            *file_reverts.entry(file.clone()).or_insert(0) += 1;
        }
    }

    let max_reverts = file_reverts
        .values()
        .cloned()
        .fold(0.0001_f64, |a, b| a.max(b as f64));

    files
        .iter()
        .map(|file| {
            let count = file_reverts.get(file).cloned().unwrap_or(0);
            (
                file.clone(),
                RevertData {
                    revert_count: count,
                    revert_score: (count as f64 / max_reverts) * 100.0,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Commit;

    fn make_commit(subject: &str, files: &[&str]) -> Commit {
        Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp: 1700000000,
            subject: subject.to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_detects_revert_commits() {
        let commits = vec![
            make_commit("Revert \"add feature\"", &["src/feature.rs"]),
            make_commit("add another feature", &["src/other.rs"]),
        ];
        let files = vec!["src/feature.rs".to_string(), "src/other.rs".to_string()];
        let result = analyze_reverts(&commits, &files);
        assert_eq!(
            result["src/feature.rs"].revert_count, 1,
            "feature.rs should have 1 revert"
        );
        assert_eq!(
            result["src/other.rs"].revert_count, 0,
            "other.rs should have 0 reverts"
        );
    }

    #[test]
    fn test_non_revert_commits_score_zero() {
        let commits = vec![
            make_commit("add feature", &["a.rs"]),
            make_commit("update something", &["a.rs"]),
        ];
        let files = vec!["a.rs".to_string()];
        let result = analyze_reverts(&commits, &files);
        assert_eq!(result["a.rs"].revert_count, 0);
        assert_eq!(result["a.rs"].revert_score, 0.0);
    }

    #[test]
    fn test_revert_detection_is_case_insensitive() {
        for prefix in &["Revert", "revert", "REVERT"] {
            let commits = vec![make_commit(&format!("{} something", prefix), &["a.rs"])];
            let files = vec!["a.rs".to_string()];
            let result = analyze_reverts(&commits, &files);
            assert!(
                result["a.rs"].revert_count > 0,
                "prefix '{}' should be detected as a revert",
                prefix
            );
        }
    }

    #[test]
    fn test_revert_score_in_range() {
        let commits = vec![
            make_commit("Revert commit A", &["a.rs"]),
            make_commit("Revert commit B", &["a.rs", "b.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let result = analyze_reverts(&commits, &files);
        for (_, data) in &result {
            assert!(
                data.revert_score >= 0.0 && data.revert_score <= 100.0,
                "revert_score {} out of range",
                data.revert_score
            );
        }
    }
}
